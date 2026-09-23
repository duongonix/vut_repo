//! Worker loop, run driver, and the wake callback installed on scheduled tasks.
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use super::Scheduler;
use crate::async_handle::{self, AsyncHandle};
use crate::task;

/// Number of worker threads for a run.
///
/// `VUT_MAXPROCS` overrides the default. An unset, zero, or invalid value falls
/// back to the number of available logical CPUs; the result is never zero.
pub(crate) fn configured_workers() -> usize {
    resolve_workers(std::env::var("VUT_MAXPROCS").ok().as_deref())
}

/// Resolves a `VUT_MAXPROCS` value. Unset, zero, or invalid values fall back to
/// the available logical CPUs; the result is never zero.
fn resolve_workers(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|workers| *workers >= 1)
        .unwrap_or_else(default_workers)
}

fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .max(1)
}

/// Runs the scheduler until `root` completes. The caller is worker 0.
///
/// # Safety
/// `root` must be a live scheduled task handle.
pub(crate) unsafe fn run(scheduler: &Arc<Scheduler>, root: *mut AsyncHandle) -> i32 {
    // One program run drives the global scheduler at a time.
    let _run = scheduler.lock_run();
    scheduler.begin_run();
    scheduler.set_root(root);
    scheduler.set_active(true);
    let workers = configured_workers();
    scheduler.configure_workers(workers);
    for index in 1..workers {
        if let Some(handle) = spawn_worker(scheduler, &format!("vut-worker-{index}")) {
            scheduler.track_worker(handle);
        }
    }
    worker_loop(scheduler);
    // Shutdown is set; join every worker (including replacements) before cleanup.
    let handles: Vec<JoinHandle<()>> = scheduler.take_workers();
    for handle in handles {
        let _ = handle.join();
    }
    scheduler.set_active(false);
    // SAFETY: `root` is a live handle owned by the caller for this run.
    i32::from(task::task_completed(unsafe { &*root }))
}

/// Spawns a worker thread that runs the poll loop.
fn spawn_worker(scheduler: &Arc<Scheduler>, name: &str) -> Option<JoinHandle<()>> {
    let worker = Arc::clone(scheduler);
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(move || worker_loop(&worker))
        .ok()
}

/// Spawns a bounded replacement worker while others are blocked in native calls.
pub(crate) fn spawn_replacement(scheduler: &Arc<Scheduler>) {
    // The check and the push happen under the same lock `run` uses to take the
    // worker set, so a replacement is never spawned after shutdown begins.
    if scheduler.is_shutdown() {
        return;
    }
    if let Some(handle) = spawn_worker(scheduler, "vut-worker-extra") {
        scheduler.track_worker(handle);
    }
}

/// Polls ready tasks until shutdown. One instance runs per worker thread.
pub(crate) fn worker_loop(scheduler: &Arc<Scheduler>) {
    let index = scheduler.claim_worker_index();
    crate::scheduler::set_worker_index(index);
    scheduler.register_worker();
    crate::scheduler::set_in_worker(true);
    let mut retired = false;
    loop {
        if scheduler.is_shutdown() {
            break;
        }
        // A surplus replacement worker retires once no worker is blocked.
        if scheduler.try_retire() {
            retired = true;
            break;
        }
        let Some(handle) = scheduler.pop_or_park(index) else {
            break;
        };
        if handle.is_null() {
            continue;
        }
        // SAFETY: deferred reclamation keeps every scheduled handle alive until
        // shutdown, so this pointer stays valid for the whole iteration.
        let handle_ref = unsafe { &*handle };
        if async_handle::handle_dropped(handle_ref) || task::task_completed(handle_ref) {
            continue;
        }
        let Some(task) = handle_ref.task() else {
            continue;
        };
        if task.running.swap(true, Ordering::AcqRel) {
            // Another worker owns it; its poll will observe any new state.
            continue;
        }
        if async_handle::handle_dropped(handle_ref) {
            // Dropped between the registry check and claiming the task.
            task.running.store(false, Ordering::Release);
            continue;
        }
        let now = scheduler.running_start();
        scheduler.note_max_running(now);
        let previous = async_handle::set_current(handle);
        // SAFETY: the handle is live; its completion wakes this same task.
        unsafe {
            async_handle::vut_rt_async_on_resume_v1(handle, handle.cast::<c_void>(), resume_task);
        }
        // SAFETY: the handle is a live task.
        let polled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            task::vut_rt_task_poll_v1(handle)
        }));
        async_handle::set_current(previous);
        scheduler.running_end();
        task.running.store(false, Ordering::Release);
        // A drop that arrived while this task was running is finished here.
        // SAFETY: the handle is live and owned by the runtime.
        unsafe { async_handle::poll_finished(handle) };
        if polled.is_err() {
            // Isolate an unexpected panic: the worker survives, and a panicking
            // root stops the run so cleanup can proceed.
            let root = scheduler.root();
            if handle == root {
                scheduler.set_shutdown();
            }
            continue;
        }
        let root = scheduler.root();
        if !root.is_null() && task::task_completed(unsafe { &*root }) {
            scheduler.set_shutdown();
        }
    }
    crate::scheduler::set_in_worker(false);
    crate::scheduler::set_worker_index(usize::MAX);
    if !retired {
        scheduler.worker_exit();
    }
}

/// Wakes the task whose handle is `ctx`.
///
/// Installed by the worker as the task's resume callback and forwarded to
/// awaited children, so completion of a dependency resumes the awaiter.
pub(crate) unsafe extern "C" fn resume_task(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    if let Some(scheduler) = super::scheduler_opt() {
        scheduler.wake(ctx.cast::<AsyncHandle>());
    }
}

#[cfg(test)]
mod tests {
    use super::{default_workers, resolve_workers};

    #[test]
    fn resolves_explicit_and_invalid_worker_counts() {
        assert_eq!(resolve_workers(Some("1")), 1);
        assert_eq!(resolve_workers(Some("4")), 4);
        assert_eq!(resolve_workers(Some(" 8 ")), 8);
        // Zero and invalid values fall back to the machine default, never zero.
        assert_eq!(resolve_workers(Some("0")), default_workers());
        assert_eq!(resolve_workers(Some("-1")), default_workers());
        assert_eq!(resolve_workers(Some("abc")), default_workers());
        assert_eq!(resolve_workers(Some("")), default_workers());
        assert_eq!(resolve_workers(None), default_workers());
        assert!(default_workers() >= 1);
    }
}
