//! Await drivers for poll tasks.
//!
//! `await` of a native future or a Vutcon task resumes the awaiting task when
//! the awaited task completes. Inside the executor this is done by forwarding
//! the currently polled task's `resume` callback onto the child; outside the
//! executor a blocking condition-variable driver plays the same role.
use std::ffi::c_void;
use std::sync::{Condvar, Mutex};

use crate::async_handle::{ASYNC_CANCELLED, ASYNC_PENDING, ASYNC_READY, AsyncHandle, CURRENT};
use crate::task::{is_task, task_completed, vut_rt_task_result_v1};

/// Forwards the currently polled task's `resume` callback onto `child`, so that
/// completion of the child resumes the current task (no lost wakeup).
fn forward_resume(child: *mut AsyncHandle) {
    let parent = CURRENT.with(std::cell::Cell::get);
    if parent.is_null() || parent == child {
        return;
    }
    // SAFETY: the parent is the task currently being polled.
    let resume = unsafe { &*parent }.lock_state().resume;
    if let Some(resume) = resume {
        // SAFETY: the callback was registered by the current task's driver.
        unsafe { crate::async_handle::vut_rt_async_register_v1(child, resume.ctx, resume.wake) };
    }
}

/// Polls an awaited child (native future or Vutcon task). If a Vutcon child is
/// not yet complete, the current task's resume callback is installed and the
/// child is left to the executor; the caller suspends. When the child finishes
/// it is dropped and `*slot` is cleared, so the owning frame's teardown never
/// double-drops it.
///
/// # Safety
/// `slot` must point to a valid child slot and `out` must have room for the
/// result type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_async_await_child_v1(
    slot: *mut *mut AsyncHandle,
    out: *mut u8,
) -> i32 {
    if slot.is_null() {
        return ASYNC_CANCELLED;
    }
    // SAFETY: the caller passes a live child slot.
    let child = unsafe { *slot };
    if child.is_null() {
        return ASYNC_CANCELLED;
    }
    forward_resume(child);
    if is_task(unsafe { &*child }) {
        if !task_completed(unsafe { &*child }) {
            // The child is eagerly scheduled; the executor completes it and then
            // resumes us through the forwarded callback.
            return ASYNC_PENDING;
        }
        // SAFETY: the child completed; move its result out and release it.
        unsafe {
            vut_rt_task_result_v1(child, out);
            crate::async_handle::vut_rt_async_drop_v1(child);
            *slot = std::ptr::null_mut();
        }
        return ASYNC_READY;
    }
    // SAFETY: the child is live and `out` is caller storage.
    let status = unsafe { crate::async_handle::vut_rt_async_poll_v1(child, out) };
    if status != ASYNC_PENDING {
        // SAFETY: the child has finished; release it and clear the slot.
        unsafe {
            crate::async_handle::vut_rt_async_drop_v1(child);
            *slot = std::ptr::null_mut();
        }
    }
    status
}

struct WaitCtx {
    ready: Mutex<bool>,
    condvar: Condvar,
}

unsafe extern "C" fn wait_notify(ctx: *mut c_void) {
    // SAFETY: `ctx` points at a live `WaitCtx` for the duration of the await.
    let ctx = unsafe { &*(ctx.cast::<WaitCtx>()) };
    let mut ready = ctx
        .ready
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *ready = true;
    drop(ready);
    ctx.condvar.notify_one();
}

/// Drives a poll task to completion, blocking the calling thread on a condition
/// variable. Used to await native futures outside the executor; a dependency may
/// complete on any thread with no busy-loop and no lost wakeup.
///
/// # Safety
/// `handle` must be a live handle and `out` must have room for the result type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_future_await_v1(handle: *mut AsyncHandle, out: *mut u8) -> i32 {
    if handle.is_null() {
        return ASYNC_CANCELLED;
    }
    let ctx = WaitCtx {
        ready: Mutex::new(false),
        condvar: Condvar::new(),
    };
    let ctx_ptr = std::ptr::from_ref(&ctx).cast_mut().cast::<c_void>();
    // SAFETY: `handle` is live and `ctx_ptr` outlives this call.
    unsafe { crate::async_handle::vut_rt_async_on_resume_v1(handle, ctx_ptr, wait_notify) };
    loop {
        // SAFETY: the handle is live and `out` is caller-provided storage.
        let status = unsafe { crate::async_handle::vut_rt_async_poll_v1(handle, out) };
        if status != ASYNC_PENDING {
            return status;
        }
        let mut ready = ctx
            .ready
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !*ready {
            ready = ctx
                .condvar
                .wait(ready)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        *ready = false;
    }
}
