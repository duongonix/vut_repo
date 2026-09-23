//! ABI facade for the global poll-task scheduler.
//!
//! The scheduling machinery lives in [`crate::scheduler`]; this module only
//! exposes the C ABI entry points used by generated code and keeps their
//! signatures stable.
use crate::async_handle::AsyncHandle;
use crate::scheduler;

/// Runs the scheduler until `root` completes. Returns the completion status.
///
/// # Safety
/// `root` must be a scheduled task handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_executor_run_v1(root: *mut AsyncHandle) -> i32 {
    if root.is_null() {
        return 0;
    }
    // SAFETY: `root` is a live scheduled task handle.
    unsafe { scheduler::run(scheduler::global(), root) }
}

/// Drops every task still registered and frees retired handles.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_executor_drain_v1() {
    if let Some(scheduler) = scheduler::scheduler_opt() {
        scheduler.drain();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_handle::{
        ASYNC_PENDING, ASYNC_READY, vut_rt_async_drop_v1, vut_rt_async_new_v1,
        vut_rt_async_signal_v1,
    };
    use crate::awaiting::vut_rt_async_await_child_v1;
    use crate::task::{vut_rt_task_new_v1, vut_rt_task_result_v1};
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    static CHILD_READY: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn poll_child(_op: *mut c_void, out: *mut u8) -> i32 {
        if CHILD_READY.load(Ordering::SeqCst) {
            // SAFETY: the test provides a valid 8-byte result buffer.
            unsafe { out.cast::<i64>().write_unaligned(7) };
            ASYNC_READY
        } else {
            ASYNC_PENDING
        }
    }

    unsafe extern "C" fn drop_none(_op: *mut c_void) {}

    /// Root task body: await the native child held in `op` (a slot pointer).
    unsafe extern "C" fn poll_root(op: *mut c_void, out: *mut u8) -> i32 {
        // SAFETY: `op` is a live `*mut AsyncHandle` slot provided by the test.
        unsafe {
            vut_rt_async_await_child_v1(op.cast::<*mut crate::async_handle::AsyncHandle>(), out)
        }
    }

    #[test]
    fn resumes_when_a_dependency_completes_on_another_thread() {
        CHILD_READY.store(false, Ordering::SeqCst);
        // SAFETY: the test provides valid callbacks and sizes.
        let child = unsafe { vut_rt_async_new_v1(std::ptr::null_mut(), poll_child, drop_none) };
        let slot = Box::into_raw(Box::new(child));
        let root = unsafe {
            vut_rt_task_new_v1(
                slot.cast(),
                poll_root,
                drop_none,
                std::mem::size_of::<i64>(),
                8,
            )
        };
        scheduler::global().schedule(root);
        let child_addr = child as usize;
        let spawned = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            CHILD_READY.store(true, Ordering::SeqCst);
            // SAFETY: the child is live and signalled once.
            unsafe { vut_rt_async_signal_v1(child_addr as *mut crate::async_handle::AsyncHandle) };
        });
        // SAFETY: `root` is a live scheduled task.
        let status = unsafe { vut_rt_executor_run_v1(root) };
        let _ = spawned.join();
        assert_eq!(status, 1);
        let mut result = 0_i64;
        // SAFETY: `root` completed and the result buffer has 8 bytes.
        unsafe { vut_rt_task_result_v1(root, std::ptr::from_mut(&mut result).cast()) };
        assert_eq!(result, 7);
        // SAFETY: `root` and `slot` are owned by the test.
        unsafe {
            vut_rt_async_drop_v1(root);
            drop(Box::from_raw(slot));
        }
    }
}
