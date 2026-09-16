//! Opaque poll tasks: the shared representation of native async operations and
//! Vutcon tasks.
//!
//! Both native asynchronous operations and `vut(...)` Vutcon tasks share one
//! poll-task representation:
//!
//! ```text
//! poll_fn(op, out) -> Pending | Ready | Cancelled
//! drop_fn(op)
//! ```
//!
//! The runtime registers a callback for two distinct events:
//!
//! * `resume` — installed by whoever drives this task (the executor or the
//!   blocking root driver). It runs when a dependency this task awaits
//!   completes, so the driver can poll the task again.
//! * `waiter` — installed by an awaiter. It runs when *this* task completes.
//!
//! A native library signals completion from any thread; a Vutcon task is
//! completed by the executor polling it. All transitions are serialized by one
//! mutex, so completion-before-register, completion-while-registering,
//! completion-after-register, duplicate signals, and drop-during-completion are
//! all safe and no wake is lost. Wake callbacks run outside the lock so they may
//! re-enter the runtime.
//!
//! Vutcon-specific result storage lives in [`crate::task`]; the await drivers
//! live in [`crate::awaiting`].
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::task::TaskStorage;

/// Poll outcome returned by [`vut_rt_async_poll_v1`].
pub const ASYNC_PENDING: i32 = 0;
pub const ASYNC_READY: i32 = 1;
pub const ASYNC_CANCELLED: i32 = 2;

thread_local! {
    /// The task currently being polled on this thread. An awaited dependency
    /// forwards this task's `resume` callback so completion resumes it.
    pub(crate) static CURRENT: std::cell::Cell<*mut AsyncHandle> =
        const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// Sets the currently polled task and returns the previous one.
pub(crate) fn set_current(handle: *mut AsyncHandle) -> *mut AsyncHandle {
    CURRENT.with(|current| current.replace(handle))
}

/// Native poll function: writes the result into `out` and returns an outcome.
pub type AsyncPollFn = unsafe extern "C" fn(op: *mut c_void, out: *mut u8) -> i32;
/// Native destructor/canceller for the operation object.
pub type AsyncDropFn = unsafe extern "C" fn(op: *mut c_void);
/// Wake callback supplied by the runtime.
pub type AsyncWakeFn = unsafe extern "C" fn(ctx: *mut c_void);

#[derive(Clone, Copy)]
pub(crate) struct Wake {
    pub(crate) ctx: *mut c_void,
    pub(crate) wake: AsyncWakeFn,
}

pub(crate) struct State {
    /// Runs when this task completes (its awaiter resumes).
    pub(crate) waiter: Option<Wake>,
    /// Runs when a dependency this task awaits completes.
    pub(crate) resume: Option<Wake>,
    pub(crate) signalled: bool,
    pub(crate) dropped: bool,
}

/// Opaque poll task handed to generated code.
pub struct AsyncHandle {
    op: *mut c_void,
    poll_fn: AsyncPollFn,
    drop_fn: AsyncDropFn,
    state: Mutex<State>,
    task: Option<TaskStorage>,
}

impl AsyncHandle {
    /// The lock guarding the waiter/resume/signalled/dropped state.
    pub(crate) fn state(&self) -> &Mutex<State> {
        &self.state
    }

    pub(crate) fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn op(&self) -> *mut c_void {
        self.op
    }

    pub(crate) fn poll_fn(&self) -> AsyncPollFn {
        self.poll_fn
    }

    pub(crate) fn task(&self) -> Option<&TaskStorage> {
        self.task.as_ref()
    }
}

static LIVE_ASYNC: AtomicUsize = AtomicUsize::new(0);

/// Returns the number of live poll tasks (native operations and Vutcons).
#[must_use]
pub fn live_async_handles() -> usize {
    LIVE_ASYNC.load(Ordering::Relaxed)
}

/// Returns the number of live poll tasks.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_async_live_count_v1() -> usize {
    live_async_handles()
}

pub(crate) fn new_handle(
    op: *mut c_void,
    poll_fn: AsyncPollFn,
    drop_fn: AsyncDropFn,
    task: Option<TaskStorage>,
) -> *mut AsyncHandle {
    let handle = Box::into_raw(Box::new(AsyncHandle {
        op,
        poll_fn,
        drop_fn,
        state: Mutex::new(State {
            waiter: None,
            resume: None,
            signalled: false,
            dropped: false,
        }),
        task,
    }));
    LIVE_ASYNC.fetch_add(1, Ordering::Relaxed);
    handle
}

/// Creates a handle wrapping a native operation object.
///
/// # Safety
/// `op` must remain valid until `drop_fn` runs; `poll_fn` writes at most one
/// result into `out` and must not block.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_new_v1(
    op: *mut c_void,
    poll_fn: AsyncPollFn,
    drop_fn: AsyncDropFn,
) -> *mut AsyncHandle {
    new_handle(op, poll_fn, drop_fn, None)
}

/// Registers the awaiter callback. If the task already completed, the callback
/// runs immediately so the wake is not lost.
///
/// # Safety
/// `handle` must be a live handle; `wake` must be a valid callback for `ctx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_register_v1(
    handle: *mut AsyncHandle,
    ctx: *mut c_void,
    wake: AsyncWakeFn,
) {
    if handle.is_null() {
        return;
    }
    let immediate = {
        // SAFETY: the handle is live and owned by the runtime.
        let handle = unsafe { &*handle };
        let mut state = handle.lock_state();
        if state.dropped {
            false
        } else if state.signalled {
            // Already complete: deliver the wake now (outside the lock).
            state.waiter = Some(Wake { ctx, wake });
            true
        } else {
            state.waiter = Some(Wake { ctx, wake });
            false
        }
    };
    if immediate {
        // SAFETY: the callback was provided by the caller for `ctx`.
        unsafe { wake(ctx) };
    }
}

/// Installs the callback that resumes this task's driver when one of its
/// awaited dependencies completes.
///
/// # Safety
/// `handle` must be a live handle; `wake` must be a valid callback for `ctx`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_on_resume_v1(
    handle: *mut AsyncHandle,
    ctx: *mut c_void,
    wake: AsyncWakeFn,
) {
    if handle.is_null() {
        return;
    }
    // SAFETY: the handle is live and owned by the runtime.
    let handle = unsafe { &*handle };
    let mut state = handle.lock_state();
    if !state.dropped {
        state.resume = Some(Wake { ctx, wake });
    }
}

/// Signals that the native operation finished; wakes the awaiter at most once.
///
/// # Safety
/// `handle` must be a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_signal_v1(handle: *mut AsyncHandle) {
    if handle.is_null() {
        return;
    }
    let woken = {
        // SAFETY: the handle is live and owned by the runtime.
        let handle = unsafe { &*handle };
        let mut state = handle.lock_state();
        if state.dropped || state.signalled {
            None
        } else {
            state.signalled = true;
            state.waiter.take()
        }
    };
    if let Some(wake) = woken {
        // SAFETY: the callback was registered by the runtime for `ctx`.
        unsafe { (wake.wake)(wake.ctx) };
    }
}

/// Polls the operation, writing a result into `out` when ready.
///
/// A polled native operation inherits the currently polled task's `resume`
/// callback, so any dependency it awaits wakes the same driver exactly as the
/// nested chain would. Vutcon tasks keep their own executor-assigned resume.
///
/// # Safety
/// `handle` must be a live handle and `out` must have room for the result type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_poll_v1(handle: *mut AsyncHandle, out: *mut u8) -> i32 {
    if handle.is_null() {
        return ASYNC_CANCELLED;
    }
    // SAFETY: the handle is live and owned by the runtime.
    let handle_ref = unsafe { &*handle };
    if handle_ref.lock_state().dropped {
        return ASYNC_CANCELLED;
    }
    let parent = CURRENT.with(std::cell::Cell::get);
    let previous = set_current(handle);
    if !parent.is_null() && parent != handle && !crate::task::is_task(handle_ref) {
        // Native operations are polled inline by the driver; propagate the
        // driver's resume callback so their nested awaits wake it.
        // SAFETY: the parent is the task currently being polled.
        let resume = unsafe { &*parent }.lock_state().resume;
        if let Some(resume) = resume {
            let mut state = handle_ref.lock_state();
            if !state.dropped {
                state.resume = Some(resume);
            }
        }
    }
    // SAFETY: the native poll function was registered for this operation.
    let status = unsafe { (handle_ref.poll_fn)(handle_ref.op, out) };
    set_current(previous);
    status
}

/// Cancels and destroys the handle. Safe to call after the operation completed
/// and safe against a concurrent completion.
///
/// # Safety
/// `handle` must be a live handle produced by [`vut_rt_async_new_v1`] or
/// [`vut_rt_task_new_v1`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_drop_v1(handle: *mut AsyncHandle) {
    if handle.is_null() {
        return;
    }
    // SAFETY: the handle is live and owned by this call.
    if crate::task::is_task(unsafe { &*handle }) {
        crate::executor::forget(handle);
    }
    let (op, drop_fn) = {
        // SAFETY: the handle is live and owned by this call.
        let handle = unsafe { &*handle };
        let mut state = handle.lock_state();
        if state.dropped {
            return;
        }
        state.dropped = true;
        state.waiter = None;
        state.resume = None;
        (handle.op, handle.drop_fn)
    };
    crate::task::release_storage(handle);
    // SAFETY: the destructor was registered for `op`.
    unsafe { drop_fn(op) };
    // SAFETY: the handle was created by `new_handle` and is dropped exactly once
    // because `dropped` guards re-entry.
    drop(unsafe { Box::from_raw(handle) });
    LIVE_ASYNC.fetch_sub(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    static WAKES: AtomicUsize = AtomicUsize::new(0);
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    static READY: AtomicBool = AtomicBool::new(false);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    unsafe extern "C" fn wake(_ctx: *mut c_void) {
        WAKES.fetch_add(1, Ordering::Relaxed);
    }

    unsafe extern "C" fn poll(_op: *mut c_void, out: *mut u8) -> i32 {
        if READY.load(Ordering::Relaxed) {
            // SAFETY: the test provides a valid one-byte out buffer.
            unsafe { *out = 7 };
            ASYNC_READY
        } else {
            ASYNC_PENDING
        }
    }

    unsafe extern "C" fn drop_op(_op: *mut c_void) {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }

    fn reset() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        WAKES.store(0, Ordering::Relaxed);
        DROPS.store(0, Ordering::Relaxed);
        READY.store(false, Ordering::Relaxed);
        guard
    }

    #[test]
    fn completion_before_register_wakes_immediately() {
        let _guard = reset();
        // SAFETY: the test provides a valid operation pointer and callbacks.
        unsafe {
            let handle = vut_rt_async_new_v1(std::ptr::null_mut(), poll, drop_op);
            vut_rt_async_signal_v1(handle);
            vut_rt_async_register_v1(handle, std::ptr::null_mut(), wake);
            assert_eq!(WAKES.load(Ordering::Relaxed), 1);
            vut_rt_async_drop_v1(handle);
        }
    }

    #[test]
    fn completion_after_register_wakes_once() {
        let _guard = reset();
        // SAFETY: as above.
        unsafe {
            let handle = vut_rt_async_new_v1(std::ptr::null_mut(), poll, drop_op);
            vut_rt_async_register_v1(handle, std::ptr::null_mut(), wake);
            vut_rt_async_signal_v1(handle);
            vut_rt_async_signal_v1(handle);
            assert_eq!(WAKES.load(Ordering::Relaxed), 1);
            vut_rt_async_drop_v1(handle);
        }
    }

    #[test]
    fn poll_reports_operation_status() {
        let _guard = reset();
        // SAFETY: as above.
        unsafe {
            let handle = vut_rt_async_new_v1(std::ptr::null_mut(), poll, drop_op);
            let mut out = 0_u8;
            assert_eq!(
                vut_rt_async_poll_v1(handle, std::ptr::from_mut(&mut out)),
                ASYNC_PENDING
            );
            READY.store(true, Ordering::Relaxed);
            assert_eq!(
                vut_rt_async_poll_v1(handle, std::ptr::from_mut(&mut out)),
                ASYNC_READY
            );
            assert_eq!(out, 7);
            vut_rt_async_drop_v1(handle);
            assert_eq!(DROPS.load(Ordering::Relaxed), 1);
        }
    }

    #[test]
    fn concurrent_completion_and_drop_are_serialized() {
        let _guard = reset();
        // SAFETY: `drop_fn` (native cancel) is required to prevent any further
        // signals after it returns, so the handle is never touched after free.
        unsafe {
            for _ in 0..64 {
                let handle = vut_rt_async_new_v1(std::ptr::null_mut(), poll, drop_op);
                vut_rt_async_register_v1(handle, std::ptr::null_mut(), wake);
                vut_rt_async_signal_v1(handle);
                vut_rt_async_drop_v1(handle);
            }
        }
        assert_eq!(DROPS.load(Ordering::Relaxed), 64);
    }
}
