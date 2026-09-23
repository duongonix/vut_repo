//! Typed channels: scheduler-aware `send`/`recv` between Vutcons.
//!
//! A channel is a reference-counted buffer of owned element values plus queues
//! of suspended operations. `send`/`recv` are native poll tasks
//! ([`crate::async_handle`]): when a channel cannot complete an operation it
//! registers the operation as a waiter and returns `Pending`, which suspends the
//! awaiting Vutcon through the scheduler instead of blocking a worker thread.
//! Completing an operation wakes the opposite waiter with
//! [`vut_rt_async_signal_v1`], so the task is re-queued and re-polled.
//!
//! Values are transferred by ownership. The caller of `send` hands over an owned
//! value (a move or a retained copy); the channel owns it until a receiver takes
//! it. Managed elements are released exactly once on drop.
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::async_handle::{
    ASYNC_PENDING, ASYNC_READY, AsyncDropFn, AsyncHandle, AsyncPollFn, new_handle,
    vut_rt_async_signal_v1,
};

type ElementRelease = unsafe extern "C" fn(*mut u8);

fn poison<T>(error: std::sync::PoisonError<T>) -> T {
    error.into_inner()
}

fn trap(message: &str) -> ! {
    // SAFETY: the message bytes outlive the call.
    unsafe { crate::abi::vut_rt_panic_v1(message.as_ptr(), message.len()) }
}

/// A reference-counted channel with a buffer and suspended-operation queues.
pub struct ManagedChannel {
    references: AtomicUsize,
    capacity: usize,
    element_size: usize,
    element_release: Option<ElementRelease>,
    inner: Mutex<Inner>,
}

struct Inner {
    buffer: VecDeque<Vec<u8>>,
    senders: VecDeque<*mut SendOp>,
    receivers: VecDeque<*mut RecvOp>,
    closed: bool,
}

/// A suspended `send`. Owns its value until a receiver takes it.
struct SendOp {
    channel: *mut ManagedChannel,
    value: Option<Vec<u8>>,
    registered: bool,
    handle: *mut AsyncHandle,
}

/// A suspended `recv`. Holds a value handed off directly by a sender.
struct RecvOp {
    channel: *mut ManagedChannel,
    pending: Option<Vec<u8>>,
    registered: bool,
    done: bool,
    handle: *mut AsyncHandle,
}

impl ManagedChannel {
    fn new(
        capacity: usize,
        element_size: usize,
        element_align: usize,
        element_retain: *const (),
        element_release: *const (),
    ) -> *mut Self {
        // The retain callback is unused: `send` transfers ownership, so the
        // channel never duplicates a value. The element alignment is irrelevant
        // because values are moved as raw bytes, never as aligned references.
        let _ = (element_retain, element_align);
        LIVE_CHANNELS.fetch_add(1, Ordering::Relaxed);
        Box::into_raw(Box::new(Self {
            references: AtomicUsize::new(1),
            capacity,
            element_size: element_size.max(1),
            element_release: release_fn(element_release),
            inner: Mutex::new(Inner {
                buffer: VecDeque::new(),
                senders: VecDeque::new(),
                receivers: VecDeque::new(),
                closed: false,
            }),
        }))
    }

    fn release_value(&self, slot: *mut u8) {
        if let Some(release) = self.element_release {
            // SAFETY: the caller passes a valid element slot.
            unsafe { release(slot) };
        }
    }

    /// Copies the raw element out of `bytes` into `out`, transferring ownership.
    fn move_out(&self, mut bytes: Vec<u8>, out: *mut u8) {
        // SAFETY: `out` has room for one element and `bytes` holds one.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_mut_ptr(), out, self.element_size) };
        // Ownership moved to `out`; do not release the element here.
    }

    /// Writes the presence flag for a completed `recv` just after the element
    /// bytes. The caller's output buffer has room for `element_size + 1` bytes.
    fn write_presence(&self, out: *mut u8, present: bool) {
        // SAFETY: `out` has room for `element_size + 1` bytes.
        unsafe { *out.add(self.element_size) = u8::from(present) };
    }
}

fn release_fn(pointer: *const ()) -> Option<ElementRelease> {
    (!pointer.is_null())
        .then(|| unsafe { std::mem::transmute::<*const (), ElementRelease>(pointer) })
}

fn retain_channel(channel: *mut ManagedChannel) {
    if !channel.is_null() {
        // SAFETY: the handle is live.
        unsafe { &*channel }
            .references
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn release_channel(channel: *mut ManagedChannel) {
    if channel.is_null() {
        return;
    }
    // SAFETY: the handle owns one reference.
    if unsafe { &*channel }
        .references
        .fetch_sub(1, Ordering::AcqRel)
        == 1
    {
        drop(unsafe { Box::from_raw(channel) });
    }
}

impl Drop for ManagedChannel {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(poison);
        for mut value in inner.buffer.drain(..) {
            self.release_value(value.as_mut_ptr());
        }
        // Pending operations hold a channel reference, so no waiter can remain.
        debug_assert!(inner.senders.is_empty() && inner.receivers.is_empty());
        LIVE_CHANNELS.fetch_sub(1, Ordering::Relaxed);
    }
}

// SAFETY: all interior state is behind a mutex and raw element storage is only
// touched under that lock; element callbacks are `Send`-safe by contract.
unsafe impl Send for ManagedChannel {}
unsafe impl Sync for ManagedChannel {}

/// Creates a channel. `capacity == 0` is an unbuffered rendezvous channel.
///
/// # Safety
/// The element layout and ownership callbacks must describe the element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_channel_new_v1(
    capacity: usize,
    element_size: usize,
    element_align: usize,
    element_retain: *const (),
    element_release: *const (),
) -> *mut ManagedChannel {
    ManagedChannel::new(
        capacity,
        element_size,
        element_align,
        element_retain,
        element_release,
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// `channel` must be null or a live channel handle.
pub unsafe extern "C" fn vut_rt_channel_retain_v1(channel: *mut ManagedChannel) {
    retain_channel(channel);
}

#[unsafe(no_mangle)]
/// # Safety
/// `channel` must be null or own one live channel reference.
pub unsafe extern "C" fn vut_rt_channel_release_v1(channel: *mut ManagedChannel) {
    release_channel(channel);
}

#[unsafe(no_mangle)]
/// # Safety
/// `channel` must be null or a live channel handle.
pub unsafe extern "C" fn vut_rt_channel_close_v1(channel: *mut ManagedChannel) {
    if channel.is_null() {
        return;
    }
    // SAFETY: the handle is live.
    let channel_ref = unsafe { &*channel };
    let (receivers, senders) = {
        let mut inner = channel_ref.inner.lock().unwrap_or_else(poison);
        if inner.closed {
            drop(inner);
            trap("close on closed channel");
        }
        inner.closed = true;
        (
            inner.receivers.drain(..).collect::<Vec<_>>(),
            inner.senders.drain(..).collect::<Vec<_>>(),
        )
    };
    // Wake every waiter; each re-polls and observes the closed channel.
    for receiver in receivers {
        // SAFETY: the op is live and owned by its task.
        unsafe { vut_rt_async_signal_v1((*receiver).handle) };
    }
    for sender in senders {
        // SAFETY: the op is live and owned by its task.
        unsafe { vut_rt_async_signal_v1((*sender).handle) };
    }
}

/// Starts a `send`. The channel owns `value` until it is taken.
///
/// # Safety
/// `channel` must be live and `value` readable for one element.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_channel_send_v1(
    channel: *mut ManagedChannel,
    value: *const u8,
) -> *mut AsyncHandle {
    if channel.is_null() || value.is_null() {
        return std::ptr::null_mut();
    }
    let channel_ref = unsafe { &*channel };
    let mut bytes = vec![0_u8; channel_ref.element_size];
    // SAFETY: `value` holds one element and `bytes` has room.
    unsafe { std::ptr::copy_nonoverlapping(value, bytes.as_mut_ptr(), channel_ref.element_size) };
    let op = Box::into_raw(Box::new(SendOp {
        channel,
        value: Some(bytes),
        registered: false,
        handle: std::ptr::null_mut(),
    }));
    let handle = new_handle(
        op.cast(),
        poll_send as AsyncPollFn,
        drop_send as AsyncDropFn,
        None,
    );
    // SAFETY: `op` is live until its drop.
    unsafe { (*op).handle = handle };
    retain_channel(channel);
    handle
}

/// Starts a `recv`. On completion the raw element is written to the poll output
/// at offset 0 and `1` (present) or `0` (closed and empty) immediately after it.
///
/// # Safety
/// `channel` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_channel_recv_v1(channel: *mut ManagedChannel) -> *mut AsyncHandle {
    if channel.is_null() {
        return std::ptr::null_mut();
    }
    let op = Box::into_raw(Box::new(RecvOp {
        channel,
        pending: None,
        registered: false,
        done: false,
        handle: std::ptr::null_mut(),
    }));
    let handle = new_handle(
        op.cast(),
        poll_recv as AsyncPollFn,
        drop_recv as AsyncDropFn,
        None,
    );
    // SAFETY: `op` is live until its drop.
    unsafe { (*op).handle = handle };
    retain_channel(channel);
    handle
}

unsafe extern "C" fn poll_send(op_ptr: *mut c_void, _out: *mut u8) -> i32 {
    // SAFETY: the runtime passes back the op it registered.
    let op = unsafe { &mut *op_ptr.cast::<SendOp>() };
    // SAFETY: the op holds a live channel reference.
    let channel = unsafe { &*op.channel };
    let mut inner = channel.inner.lock().unwrap_or_else(poison);
    // Re-polled after a receiver already took the value: the send is complete.
    if op.value.is_none() {
        return ASYNC_READY;
    }
    if inner.closed {
        drop(inner);
        trap("send on closed channel");
    }
    // A waiting receiver takes the value directly (rendezvous).
    if let Some(receiver) = inner.receivers.pop_front() {
        let value = op.value.take().expect("send value present");
        // SAFETY: the receiver op is live and owned by its task.
        unsafe { (*receiver).pending = Some(value) };
        let handle = unsafe { (*receiver).handle };
        drop(inner);
        // SAFETY: the handle is live.
        unsafe { vut_rt_async_signal_v1(handle) };
        return ASYNC_READY;
    }
    // Buffered: enqueue while there is room.
    if inner.buffer.len() < channel.capacity {
        inner
            .buffer
            .push_back(op.value.take().expect("send value present"));
        return ASYNC_READY;
    }
    // Full (or unbuffered with no receiver): suspend the sender.
    if !op.registered {
        inner.senders.push_back(op);
        op.registered = true;
    }
    ASYNC_PENDING
}

unsafe extern "C" fn poll_recv(op_ptr: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: the runtime passes back the op it registered.
    let op = unsafe { &mut *op_ptr.cast::<RecvOp>() };
    // SAFETY: the op holds a live channel reference.
    let channel = unsafe { &*op.channel };
    let mut inner = channel.inner.lock().unwrap_or_else(poison);
    if op.done {
        return ASYNC_READY;
    }
    // A value handed off directly by a sender.
    if let Some(value) = op.pending.take() {
        op.done = true;
        channel.move_out(value, out);
        channel.write_presence(out, true);
        return ASYNC_READY;
    }
    // A buffered value, freeing a slot for a waiting sender.
    if let Some(value) = inner.buffer.pop_front() {
        let wake = inner.senders.pop_front().map(|sender| {
            // SAFETY: the sender op is live and owned by its task.
            let next = unsafe { (*sender).value.take().expect("sender value present") };
            inner.buffer.push_back(next);
            // SAFETY: the op is live.
            unsafe { (*sender).handle }
        });
        drop(inner);
        if let Some(handle) = wake {
            // SAFETY: the handle is live.
            unsafe { vut_rt_async_signal_v1(handle) };
        }
        channel.move_out(value, out);
        channel.write_presence(out, true);
        op.done = true;
        return ASYNC_READY;
    }
    if inner.closed {
        channel.write_presence(out, false);
        op.done = true;
        return ASYNC_READY;
    }
    // Unbuffered rendezvous with a waiting sender.
    if let Some(sender) = inner.senders.pop_front() {
        // SAFETY: the sender op is live and owned by its task.
        let value = unsafe { (*sender).value.take().expect("sender value present") };
        let handle = unsafe { (*sender).handle };
        drop(inner);
        // SAFETY: the handle is live.
        unsafe { vut_rt_async_signal_v1(handle) };
        channel.move_out(value, out);
        channel.write_presence(out, true);
        op.done = true;
        return ASYNC_READY;
    }
    if !op.registered {
        inner.receivers.push_back(op);
        op.registered = true;
    }
    ASYNC_PENDING
}

unsafe extern "C" fn drop_send(op_ptr: *mut c_void) {
    // SAFETY: the op is owned by this call and freed exactly once.
    let op = unsafe { Box::from_raw(op_ptr.cast::<SendOp>()) };
    // SAFETY: the op holds a live channel reference.
    let channel = unsafe { &*op.channel };
    // Remove the op from the waiter queue so no signal reaches freed memory.
    {
        let mut inner = channel.inner.lock().unwrap_or_else(poison);
        inner
            .senders
            .retain(|queued| std::ptr::eq(*queued, op_ptr.cast()));
    }
    if let Some(mut value) = op.value {
        channel.release_value(value.as_mut_ptr());
    }
    release_channel(op.channel);
}

unsafe extern "C" fn drop_recv(op_ptr: *mut c_void) {
    // SAFETY: the op is owned by this call and freed exactly once.
    let op = unsafe { Box::from_raw(op_ptr.cast::<RecvOp>()) };
    // SAFETY: the op holds a live channel reference.
    let channel = unsafe { &*op.channel };
    {
        let mut inner = channel.inner.lock().unwrap_or_else(poison);
        inner
            .receivers
            .retain(|queued| std::ptr::eq(*queued, op_ptr.cast()));
    }
    if let Some(mut value) = op.pending {
        channel.release_value(value.as_mut_ptr());
    }
    release_channel(op.channel);
}

/// Number of live channels (test instrumentation).
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_channel_live_count_v1() -> usize {
    LIVE_CHANNELS.load(Ordering::Relaxed)
}

static LIVE_CHANNELS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_release_are_balanced() {
        let before = vut_rt_channel_live_count_v1();
        // SAFETY: a trivial element type with no callbacks.
        let channel = unsafe { vut_rt_channel_new_v1(1, 8, 8, std::ptr::null(), std::ptr::null()) };
        assert_eq!(vut_rt_channel_live_count_v1(), before + 1);
        // SAFETY: the handle owns one reference.
        unsafe { vut_rt_channel_release_v1(channel) };
        assert_eq!(vut_rt_channel_live_count_v1(), before);
    }
}
