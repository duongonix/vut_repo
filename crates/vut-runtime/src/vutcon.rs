//! Vutcon tasks: eager-scheduled poll tasks created by `vut(...)`.
//!
//! A Vutcon is a poll task ([`crate::async_handle`]) with its own result buffer,
//! registered with the single-thread executor ([`crate::executor`]) the moment
//! it is created. Sync and async callables converge on the same representation:
//! the spawn entry receives a `(op, out) -> i32` poll function, a drop function,
//! and the result size/alignment. A sync callable's poll thunk runs the body and
//! yields `Ready` on its first poll; an async callable's poll body may yield
//! `Pending` and be resumed.
use std::ffi::c_void;

use crate::async_handle::{AsyncDropFn, AsyncHandle, AsyncPollFn};
use crate::task::vut_rt_task_new_v1;

/// Creates a Vutcon task and schedules it immediately.
///
/// # Safety
/// `poll_fn` must write the result into the buffer it is given and `drop_fn`
/// (when non-null) must release `op` exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_vutcon_spawn_v1(
    op: *mut c_void,
    poll_fn: AsyncPollFn,
    drop_fn: AsyncDropFn,
    size: usize,
    align: usize,
) -> *mut AsyncHandle {
    // SAFETY: the caller guarantees the callbacks and result layout.
    let handle = unsafe { vut_rt_task_new_v1(op, poll_fn, drop_fn, size, align) };
    crate::scheduler::global().schedule(handle);
    handle
}

/// Returns the number of live poll tasks (native operations and Vutcons).
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_vutcon_live_count_v1() -> usize {
    crate::async_handle::live_async_handles()
}
