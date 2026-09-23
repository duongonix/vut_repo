//! Channel runtime contract: buffered/unbuffered send/recv, close, FIFO,
//! scheduler-aware suspension, and value release.
#![allow(unused_unsafe)]
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use vut_runtime::AsyncHandle;
use vut_runtime::abi;

static LOCK: Mutex<()> = Mutex::new(());

const READY: i32 = 1;
const PENDING: i32 = 0;

fn new_channel(capacity: usize) -> *mut abi::ManagedChannel {
    // SAFETY: 8-byte trivial elements with no ownership callbacks.
    unsafe { abi::vut_rt_channel_new_v1(capacity, 8, 8, std::ptr::null(), std::ptr::null()) }
}

unsafe fn poll(op: *mut AsyncHandle, out: *mut u8) -> i32 {
    // SAFETY: the caller passes a live op and a large enough out buffer.
    unsafe { abi::vut_rt_async_poll_v1(op, out) }
}

unsafe fn send(channel: *mut abi::ManagedChannel, value: i64) -> *mut AsyncHandle {
    // SAFETY: `value` is one 8-byte element.
    unsafe { abi::vut_rt_channel_send_v1(channel, std::ptr::from_ref(&value).cast()) }
}

/// A completed `recv` result: the element bytes plus the presence flag written
/// immediately after them.
struct Recv {
    value: i64,
    present: bool,
}

fn read_recv(buffer: &[u8; 16]) -> Recv {
    Recv {
        value: i64::from_ne_bytes(buffer[0..8].try_into().unwrap()),
        present: buffer[8] != 0,
    }
}

unsafe fn recv_once(channel: *mut abi::ManagedChannel) -> Recv {
    let mut buffer = [0_u8; 16];
    let op = unsafe { abi::vut_rt_channel_recv_v1(channel) };
    assert_eq!(unsafe { poll(op, buffer.as_mut_ptr()) }, READY);
    unsafe { abi::vut_rt_async_drop_v1(op) };
    read_recv(&buffer)
}

#[test]
fn buffered_send_recv_is_fifo() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel = new_channel(3);
        for value in [1_i64, 2, 3] {
            let op = send(channel, value);
            assert_eq!(poll(op, std::ptr::null_mut()), READY);
            abi::vut_rt_async_drop_v1(op);
        }
        for expected in [1_i64, 2, 3] {
            let received = recv_once(channel);
            assert!(received.present);
            assert_eq!(received.value, expected);
        }
        abi::vut_rt_channel_release_v1(channel);
    }
}

#[test]
fn unbuffered_recv_waits_then_rendezvous() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel = new_channel(0);
        let mut buffer = [0_u8; 16];
        let recv = abi::vut_rt_channel_recv_v1(channel);
        // No sender yet: the receiver suspends.
        assert_eq!(poll(recv, buffer.as_mut_ptr()), PENDING);
        // A send rendezvouses with the waiting receiver.
        let snd = send(channel, 7);
        assert_eq!(poll(snd, std::ptr::null_mut()), READY);
        abi::vut_rt_async_drop_v1(snd);
        // The receiver now completes.
        assert_eq!(poll(recv, buffer.as_mut_ptr()), READY);
        let received = read_recv(&buffer);
        assert!(received.present);
        assert_eq!(received.value, 7);
        abi::vut_rt_async_drop_v1(recv);
        abi::vut_rt_channel_release_v1(channel);
    }
}

#[test]
fn buffered_send_suspends_when_full_and_wakes_on_recv() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel = new_channel(1);
        let first = send(channel, 10);
        assert_eq!(poll(first, std::ptr::null_mut()), READY);
        abi::vut_rt_async_drop_v1(first);
        // Buffer full: the next send suspends.
        let second = send(channel, 20);
        assert_eq!(poll(second, std::ptr::null_mut()), PENDING);
        // A recv frees a slot and admits the suspended sender.
        let received = recv_once(channel);
        assert_eq!(received.value, 10);
        // The sender is now admitted.
        assert_eq!(poll(second, std::ptr::null_mut()), READY);
        abi::vut_rt_async_drop_v1(second);
        // The admitted value is buffered.
        let received = recv_once(channel);
        assert_eq!(received.value, 20);
        abi::vut_rt_channel_release_v1(channel);
    }
}

#[test]
fn close_wakes_receiver_with_absence() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel = new_channel(0);
        let mut buffer = [0_u8; 16];
        let recv = abi::vut_rt_channel_recv_v1(channel);
        assert_eq!(poll(recv, buffer.as_mut_ptr()), PENDING);
        abi::vut_rt_channel_close_v1(channel);
        assert_eq!(poll(recv, buffer.as_mut_ptr()), READY);
        assert!(!read_recv(&buffer).present, "closed and empty is absence");
        abi::vut_rt_async_drop_v1(recv);
        abi::vut_rt_channel_release_v1(channel);
    }
}

#[test]
fn closed_buffered_channel_drains_then_reports_absence() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel = new_channel(4);
        for value in [1_i64, 2] {
            let op = send(channel, value);
            assert_eq!(poll(op, std::ptr::null_mut()), READY);
            abi::vut_rt_async_drop_v1(op);
        }
        abi::vut_rt_channel_close_v1(channel);
        for expected in [1_i64, 2] {
            let received = recv_once(channel);
            assert!(received.present);
            assert_eq!(received.value, expected);
        }
        assert!(!recv_once(channel).present);
        abi::vut_rt_channel_release_v1(channel);
    }
}

static RELEASED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn count_release(_slot: *mut u8) {
    RELEASED.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn dropping_channel_releases_buffered_values_once() {
    let _guard = LOCK.lock().unwrap();
    RELEASED.store(0, Ordering::SeqCst);
    // SAFETY: the handles are live for the duration of the test.
    unsafe {
        let channel =
            abi::vut_rt_channel_new_v1(4, 8, 8, std::ptr::null(), count_release as *const ());
        for value in [1_i64, 2, 3] {
            let op = send(channel, value);
            assert_eq!(poll(op, std::ptr::null_mut()), READY);
            abi::vut_rt_async_drop_v1(op);
        }
        abi::vut_rt_channel_release_v1(channel);
    }
    assert_eq!(RELEASED.load(Ordering::SeqCst), 3);
}

// ---- Scheduler-aware suspension through the executor ----

unsafe extern "C" fn drop_none(_op: *mut c_void) {}

/// Task body: await the channel recv op held in the `op` slot.
unsafe extern "C" fn poll_root(op: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: `op` points at a live `*mut AsyncHandle` slot.
    unsafe { abi::vut_rt_async_await_child_v1(op.cast::<*mut AsyncHandle>(), out) }
}

#[test]
fn recv_suspends_task_and_a_sender_thread_wakes_it() {
    let _guard = LOCK.lock().unwrap();
    // SAFETY: every handle is live for the duration of the test.
    unsafe {
        let channel = new_channel(0);
        let recv = abi::vut_rt_channel_recv_v1(channel);
        let slot = Box::into_raw(Box::new(recv));
        // Create and schedule the root task; its body awaits the recv op. The
        // result buffer covers the element plus the presence byte.
        let root = abi::vut_rt_vutcon_spawn_v1(slot.cast(), poll_root, drop_none, 16, 8);

        // Another thread sends after a delay; the executor parks meanwhile.
        let channel_address = channel as usize;
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            let channel = channel_address as *mut abi::ManagedChannel;
            let value = 99_i64;
            // SAFETY: the channel is live (the test holds a reference).
            let op =
                unsafe { abi::vut_rt_channel_send_v1(channel, std::ptr::from_ref(&value).cast()) };
            loop {
                // SAFETY: the op is live until dropped.
                if unsafe { poll(op, std::ptr::null_mut()) } == READY {
                    break;
                }
                std::thread::yield_now();
            }
            // SAFETY: the op is owned by this thread.
            unsafe { abi::vut_rt_async_drop_v1(op) };
        });

        // SAFETY: `root` is a scheduled task handle.
        let status = unsafe { abi::vut_rt_executor_run_v1(root) };
        sender.join().expect("sender");
        assert_eq!(status, 1, "the root task must complete");

        let mut buffer = [0_u8; 16];
        // SAFETY: `root` completed and the result buffer has 16 bytes.
        unsafe { abi::vut_rt_task_result_v1(root, buffer.as_mut_ptr()) };
        let received = read_recv(&buffer);
        assert!(received.present);
        assert_eq!(received.value, 99);
        // SAFETY: the handles are owned by the test.
        unsafe {
            abi::vut_rt_async_drop_v1(root);
            drop(Box::from_raw(slot));
            abi::vut_rt_channel_release_v1(channel);
        }
    }
}
