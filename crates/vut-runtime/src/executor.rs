//! Single-threaded poll-task executor with a thread-safe wake seam.
//!
//! The executor owns a ready set of tasks and runs them on one OS thread. Each
//! task is polled through [`crate::async_handle`]; when a task is polled the
//! executor installs a `resume` callback so that completion of any dependency it
//! awaits (a native operation or another Vutcon) re-schedules the task.
//!
//! Native libraries may complete work on other threads; they only signal/wake,
//! which re-schedules a task and notifies the executor. The executor blocks on a
//! condition variable when idle, so there is no busy-waiting and no lost wakeup.
//! It never runs two tasks in parallel.
//!
//! The executor state lives in a thread-local cell that is only borrowed
//! briefly: polling a task re-enters the runtime (it may schedule or drop
//! tasks), so no borrow is held across a poll.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};

use crate::async_handle::AsyncHandle;
use crate::task::{set_task_index, task_completed, task_index};

/// Shared ready set and idle condition variable, reachable from any thread that
/// wakes a task.
struct Shared {
    ready: Mutex<VecDeque<usize>>,
    condvar: Condvar,
}

/// A task's re-scheduling callback context. Boxed so its address is stable for
/// the whole run, even as the executor's task list grows.
struct Resume {
    shared: Arc<Shared>,
    index: usize,
}

unsafe extern "C" fn resume_task(ctx: *mut c_void) {
    // SAFETY: `ctx` points at a live `Resume` owned by the executor.
    let resume = unsafe { &*ctx.cast::<Resume>() };
    let mut ready = resume
        .shared
        .ready
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    ready.push_back(resume.index);
    drop(ready);
    resume.shared.condvar.notify_one();
}

struct Executor {
    shared: Arc<Shared>,
    tasks: Vec<*mut AsyncHandle>,
    resumes: Vec<*mut Resume>,
}

thread_local! {
    static EXECUTOR: RefCell<Executor> = RefCell::new(Executor {
        shared: Arc::new(Shared {
            ready: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        }),
        tasks: Vec::new(),
        resumes: Vec::new(),
    });
}

/// Registers `handle` with the executor and schedules it immediately.
pub fn schedule(handle: *mut AsyncHandle) -> usize {
    let (index, shared) = EXECUTOR.with(|executor| {
        let mut executor = executor.borrow_mut();
        let index = executor.tasks.len();
        executor.tasks.push(handle);
        // Boxed so the callback context has a stable address for the whole run,
        // even as the registry grows.
        let resume = Box::into_raw(Box::new(Resume {
            shared: Arc::clone(&executor.shared),
            index,
        }));
        executor.resumes.push(resume);
        set_task_index(handle, index);
        (index, Arc::clone(&executor.shared))
    });
    shared
        .ready
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push_back(index);
    shared.condvar.notify_one();
    index
}

/// Removes a dropped task from the executor registry.
pub fn forget(handle: *mut AsyncHandle) {
    EXECUTOR.with(|executor| {
        let mut executor = executor.borrow_mut();
        if let Some(index) = task_index(handle)
            && let Some(slot) = executor.tasks.get_mut(index)
        {
            *slot = std::ptr::null_mut();
        }
    });
}

/// Extracts the raw `resume` callback pointer for a scheduled task.
fn resume_pointer(index: usize) -> *mut c_void {
    EXECUTOR.with(|executor| {
        let executor = executor.borrow();
        executor
            .resumes
            .get(index)
            .copied()
            .unwrap_or(std::ptr::null_mut())
            .cast::<c_void>()
    })
}

/// Pops the next ready task index, if any.
fn pop_ready() -> Option<usize> {
    EXECUTOR.with(|executor| {
        executor
            .borrow()
            .shared
            .ready
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
    })
}

/// Returns the current handle for a scheduled task index.
fn task_handle(index: usize) -> *mut AsyncHandle {
    EXECUTOR.with(|executor| {
        executor
            .borrow()
            .tasks
            .get(index)
            .copied()
            .unwrap_or(std::ptr::null_mut())
    })
}

/// Blocks until a task is woken. Does not hold the executor borrow while idle.
fn wait_for_wake(root: *mut AsyncHandle) {
    let shared = EXECUTOR.with(|executor| Arc::clone(&executor.borrow().shared));
    let ready = shared
        .ready
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: `root` is non-null and live for the caller's duration.
    if ready.is_empty() && !task_completed(unsafe { &*root }) {
        drop(
            shared
                .condvar
                .wait(ready)
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
    }
}

/// Runs the executor until `root` completes. Returns the completion status.
///
/// # Safety
/// `root` must be a scheduled task handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_executor_run_v1(root: *mut AsyncHandle) -> i32 {
    if root.is_null() {
        return 0;
    }
    // SAFETY: `root` is non-null and live for the caller's duration.
    while !task_completed(unsafe { &*root }) {
        let Some(index) = pop_ready() else {
            wait_for_wake(root);
            continue;
        };
        let handle = task_handle(index);
        if handle.is_null() {
            continue;
        }
        // SAFETY: `handle` is non-null and live for the caller's duration.
        if task_completed(unsafe { &*handle }) {
            continue;
        }
        let resume = resume_pointer(index);
        // SAFETY: `handle` is a live task and `resume` outlives the run.
        unsafe { crate::async_handle::vut_rt_async_on_resume_v1(handle, resume, resume_task) };
        // SAFETY: `handle` is a live task. Polling may schedule or drop tasks.
        unsafe { crate::task::vut_rt_task_poll_v1(handle) };
    }
    // SAFETY: `root` is non-null and live for the caller's duration.
    i32::from(task_completed(unsafe { &*root }))
}

/// Cancels and releases every task still registered with the executor.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_executor_drain_v1() {
    let (detached, resumes) = EXECUTOR.with(|executor| {
        let mut executor = executor.borrow_mut();
        (
            std::mem::take(&mut executor.tasks),
            std::mem::take(&mut executor.resumes),
        )
    });
    for handle in detached {
        if !handle.is_null() {
            // SAFETY: each handle was produced by the task spawn path and is
            // dropped exactly once here.
            unsafe { crate::async_handle::vut_rt_async_drop_v1(handle) };
        }
    }
    for resume in resumes {
        if !resume.is_null() {
            // SAFETY: each resume context was boxed by `schedule` and is freed
            // once here, after no task can be woken again.
            unsafe { drop(Box::from_raw(resume)) };
        }
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
        unsafe { vut_rt_async_await_child_v1(op.cast::<*mut AsyncHandle>(), out) }
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
        schedule(root);
        let child_addr = child as usize;
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            CHILD_READY.store(true, Ordering::SeqCst);
            // SAFETY: the child is live and signalled once.
            unsafe { vut_rt_async_signal_v1(child_addr as *mut AsyncHandle) };
        });
        // SAFETY: `root` is a live scheduled task.
        let status = unsafe { vut_rt_executor_run_v1(root) };
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
