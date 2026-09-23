//! Vutcon task storage: result buffer, completion flag, and scheduler identity.
//!
//! A Vutcon is a poll task (see [`crate::async_handle`]) that additionally owns
//! the storage for its result and the scheduler identity it was scheduled with.
//! This module owns that task-specific state; the poll/wake representation
//! itself lives in `async_handle`.
//!
//! The scheduler uses two per-task guards:
//!
//! * `queued` — the task has an entry in the scheduler ready queue, so a wake
//!   must not enqueue a second copy.
//! * `running` — a worker is currently polling the task, so no other worker may
//!   poll it and a concurrent drop must defer its destructor.
use std::alloc::{Layout, alloc, dealloc};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::async_handle::{
    ASYNC_CANCELLED, ASYNC_PENDING, ASYNC_READY, AsyncDropFn, AsyncHandle, AsyncPollFn, new_handle,
};

/// Sentinel for "not scheduled yet".
const NO_ID: usize = usize::MAX;

/// Result storage owned by a Vutcon task. Native operations have none.
pub(crate) struct TaskStorage {
    buffer: *mut u8,
    size: usize,
    align: usize,
    pub(crate) completed: AtomicBool,
    /// Global scheduler id, assigned when the task is scheduled.
    pub(crate) id: AtomicUsize,
    /// `true` while the task has an entry in the scheduler ready queue.
    pub(crate) queued: AtomicBool,
    /// `true` while a worker is polling the task.
    pub(crate) running: AtomicBool,
}

fn result_layout(size: usize, align: usize) -> Layout {
    Layout::from_size_align(size.max(1), align.max(1)).expect("task result layout")
}

/// Creates a task handle that owns a result buffer. Used for `vut(...)`.
///
/// # Safety
/// `op` must remain valid until `drop_fn` runs; `poll_fn` writes the task's
/// result into the buffer it is given.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_task_new_v1(
    op: *mut c_void,
    poll_fn: AsyncPollFn,
    drop_fn: AsyncDropFn,
    size: usize,
    align: usize,
) -> *mut AsyncHandle {
    let buffer = if size == 0 {
        std::ptr::null_mut()
    } else {
        // SAFETY: the caller provides a valid size/alignment for the result type.
        unsafe { alloc(result_layout(size, align)) }
    };
    new_handle(
        op,
        poll_fn,
        drop_fn,
        Some(TaskStorage {
            buffer,
            size,
            align,
            completed: AtomicBool::new(false),
            id: AtomicUsize::new(NO_ID),
            queued: AtomicBool::new(false),
            running: AtomicBool::new(false),
        }),
    )
}

/// Returns whether `handle` is a Vutcon task (owns result storage).
#[must_use]
pub fn is_task(handle: &AsyncHandle) -> bool {
    handle.task().is_some()
}

/// Returns whether a Vutcon task has completed.
#[must_use]
pub fn task_completed(handle: &AsyncHandle) -> bool {
    handle
        .task()
        .is_some_and(|task| task.completed.load(Ordering::Acquire))
}

/// The scheduler id of a task, or `None` before it is scheduled.
pub(crate) fn task_id(handle: *mut AsyncHandle) -> Option<usize> {
    // SAFETY: the handle is live for the caller's duration.
    let task = unsafe { &*handle }.task()?;
    let id = task.id.load(Ordering::Relaxed);
    (id != NO_ID).then_some(id)
}

pub(crate) fn set_task_id(handle: *mut AsyncHandle, id: usize) {
    // SAFETY: the handle is live for the caller's duration.
    if let Some(task) = unsafe { &*handle }.task() {
        task.id.store(id, Ordering::Relaxed);
    }
}

/// Frees a task's result buffer, if any. Called once from handle teardown.
pub(crate) fn release_storage(handle: *mut AsyncHandle) {
    // SAFETY: the handle is live and owned by the caller.
    let Some(task) = (unsafe { &*handle }).task() else {
        return;
    };
    if !task.buffer.is_null() {
        // SAFETY: the buffer was allocated by `vut_rt_task_new_v1` with this
        // layout and is not used after this point.
        unsafe { dealloc(task.buffer, result_layout(task.size, task.align)) };
    }
}

/// Polls a Vutcon task, writing its result into the task's own buffer. On
/// completion the task is marked done and its awaiter is woken.
///
/// # Safety
/// `handle` must be a live task handle.
pub(crate) unsafe fn poll_task(handle: *mut AsyncHandle) -> i32 {
    // SAFETY: the handle is live and owned by the runtime.
    let handle_ref = unsafe { &*handle };
    let Some(task) = handle_ref.task() else {
        return ASYNC_CANCELLED;
    };
    if task.completed.load(Ordering::Acquire) {
        return ASYNC_READY;
    }
    let previous = crate::async_handle::set_current(handle);
    // SAFETY: the poll function was registered for this task.
    let status = unsafe { (handle_ref.poll_fn())(handle_ref.op(), task.buffer) };
    crate::async_handle::set_current(previous);
    if status != ASYNC_PENDING {
        task.completed.store(true, Ordering::Release);
        let woken = {
            let mut state = handle_ref
                .state()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if status == ASYNC_CANCELLED {
                state.signalled = true;
            }
            state.waiter.take()
        };
        if let Some(wake) = woken {
            // SAFETY: the callback was registered by the awaiter for `ctx`.
            unsafe { (wake.wake)(wake.ctx) };
        }
    }
    status
}

/// Drives a Vutcon task one step (executor entry point).
///
/// # Safety
/// `handle` must be a live task handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_task_poll_v1(handle: *mut AsyncHandle) -> i32 {
    if handle.is_null() {
        return ASYNC_CANCELLED;
    }
    unsafe { poll_task(handle) }
}

/// Copies a completed task's result into `destination`.
///
/// # Safety
/// `handle` must be a completed task and `destination` must have room for it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_task_result_v1(handle: *mut AsyncHandle, destination: *mut u8) {
    if handle.is_null() || destination.is_null() {
        return;
    }
    // SAFETY: the handle is live and owned by the caller.
    let Some(task) = (unsafe { &*handle }).task() else {
        return;
    };
    if task.size != 0 && !task.buffer.is_null() {
        // SAFETY: source and destination both hold `size` initialized bytes; the
        // copy transfers ownership of any managed contents to the caller.
        unsafe { std::ptr::copy_nonoverlapping(task.buffer, destination, task.size) };
    }
}
