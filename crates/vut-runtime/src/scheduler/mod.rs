//! Global poll-task scheduler shared by every worker thread.
//!
//! The scheduler owns the ready queue and a task registry. Tasks are raw
//! [`AsyncHandle`] pointers; because reclamation is deferred to shutdown, a
//! handle stays alive for the whole run, so a stale ready entry can never dangle.
//!
//! Reentrancy: a polled task may call back into the runtime (spawn, wake, drop).
//! No scheduler lock is ever held across a poll, so those calls cannot deadlock.
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use crate::async_handle::AsyncHandle;
use crate::task;

mod worker;
pub(crate) use worker::{configured_workers, run};

fn poisoned<T>(error: std::sync::PoisonError<T>) -> T {
    error.into_inner()
}

/// Ready work: a global injector plus one local queue per worker.
///
/// New tasks and cross-thread wakes enter the injector; a wake raised while a
/// worker is polling enters that worker's local queue. Workers pop their local
/// queue last-in-first-out, drain the injector first-in-first-out, then steal
/// from other workers' local queues. All queue access is serialized by the
/// scheduler's single `queues` lock, which also backs parking.
struct Queues {
    injector: VecDeque<*mut AsyncHandle>,
    locals: Vec<VecDeque<*mut AsyncHandle>>,
}

impl Queues {
    fn new() -> Self {
        Self {
            injector: VecDeque::new(),
            locals: Vec::new(),
        }
    }

    fn resize(&mut self, workers: usize) {
        while self.locals.len() < workers {
            self.locals.push(VecDeque::new());
        }
    }

    fn push_local(&mut self, index: usize, handle: *mut AsyncHandle) {
        if let Some(local) = self.locals.get_mut(index) {
            local.push_back(handle);
        } else {
            self.injector.push_back(handle);
        }
    }

    /// Drops every queued pointer. Called only when no worker is running, so no
    /// queued handle can be in use.
    fn clear(&mut self) {
        self.injector.clear();
        for local in &mut self.locals {
            local.clear();
        }
    }

    /// Pops work for `index`: local LIFO, then injector FIFO, then steal.
    fn pop(&mut self, index: usize) -> Option<*mut AsyncHandle> {
        if let Some(local) = self.locals.get_mut(index)
            && let Some(handle) = local.pop_back()
        {
            return Some(handle);
        }
        if let Some(handle) = self.injector.pop_front() {
            return Some(handle);
        }
        let workers = self.locals.len();
        for offset in 1..workers {
            let victim = (index + offset) % workers;
            if let Some(local) = self.locals.get_mut(victim)
                && let Some(handle) = local.pop_front()
            {
                return Some(handle);
            }
        }
        None
    }
}

/// Global scheduler state.
pub(crate) struct Scheduler {
    /// Ready work: injector + per-worker local queues. A task appears at most
    /// once while awake.
    queues: Mutex<Queues>,
    /// Parked workers are woken through this condition variable.
    park: Condvar,
    /// Live scheduled tasks, keyed by id, until shutdown/drain.
    registry: Mutex<HashMap<usize, *mut AsyncHandle>>,
    /// Handles dropped during a run; freed once no worker can reference them.
    retired: Mutex<Vec<*mut AsyncHandle>>,
    /// Serializes `run`/`drain` so only one program run drives the global
    /// scheduler at a time (production has one entry; tests may run in parallel).
    run_lock: Mutex<()>,
    next_id: AtomicUsize,
    /// Next worker local-queue index to hand out for the run.
    next_index: AtomicUsize,
    shutdown: AtomicBool,
    active: AtomicBool,
    root: AtomicPtr<AsyncHandle>,
    /// Instrumentation: tasks currently executing a poll.
    running: AtomicUsize,
    max_running: AtomicUsize,
    /// Worker-pool accounting for blocking offload.
    desired_workers: AtomicUsize,
    total_workers: AtomicUsize,
    blocked_workers: AtomicUsize,
    max_workers: AtomicUsize,
    /// Join handles for every worker (initial and replacement) of the run.
    workers: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

// SAFETY: the scheduler serializes all access to its raw task pointers through
// its own mutexes and atomics, and deferred reclamation keeps every handle alive
// until the run has stopped, so a task pointer is never used concurrently
// without synchronization.
unsafe impl Send for Scheduler {}
// SAFETY: as above; shared access only ever mutates scheduler-owned state under
// the scheduler's locks.
unsafe impl Sync for Scheduler {}

impl Scheduler {
    fn new() -> Self {
        Self {
            queues: Mutex::new(Queues::new()),
            park: Condvar::new(),
            registry: Mutex::new(HashMap::new()),
            retired: Mutex::new(Vec::new()),
            run_lock: Mutex::new(()),
            next_id: AtomicUsize::new(0),
            next_index: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            active: AtomicBool::new(false),
            root: AtomicPtr::new(std::ptr::null_mut()),
            running: AtomicUsize::new(0),
            max_running: AtomicUsize::new(0),
            desired_workers: AtomicUsize::new(1),
            total_workers: AtomicUsize::new(0),
            blocked_workers: AtomicUsize::new(0),
            max_workers: AtomicUsize::new(1),
            workers: Mutex::new(Vec::new()),
        }
    }

    /// Registers a task and schedules its first poll.
    pub(crate) fn schedule(&self, handle: *mut AsyncHandle) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        task::set_task_id(handle, id);
        self.registry
            .lock()
            .unwrap_or_else(poisoned)
            .insert(id, handle);
        self.wake(handle);
    }

    /// Enqueues a task for one more poll, coalescing duplicate wakes.
    pub(crate) fn wake(&self, handle: *mut AsyncHandle) {
        if handle.is_null() {
            return;
        }
        // SAFETY: the handle is kept alive until shutdown by deferred reclamation.
        let Some(task) = (unsafe { handle.as_ref() }).and_then(AsyncHandle::task) else {
            return;
        };
        if task.completed.load(Ordering::Acquire) {
            return;
        }
        if task.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        // A wake raised while polling goes to this worker's local queue; any
        // other wake goes to the shared injector.
        let local = if in_worker() {
            Some(current_worker_index())
        } else {
            None
        };
        let mut queues = self.queues.lock().unwrap_or_else(poisoned);
        match local {
            Some(index) => queues.push_local(index, handle),
            None => queues.injector.push_back(handle),
        }
        drop(queues);
        self.park.notify_one();
    }

    /// Pops the next ready task for worker `index`, parking until one is
    /// available or shutdown.
    pub(crate) fn pop_or_park(&self, index: usize) -> Option<*mut AsyncHandle> {
        let mut queues = self.queues.lock().unwrap_or_else(poisoned);
        loop {
            if let Some(handle) = queues.pop(index) {
                // SAFETY: the handle is kept alive until shutdown.
                if let Some(task) = unsafe { handle.as_ref() }.and_then(AsyncHandle::task) {
                    task.queued.store(false, Ordering::Release);
                }
                return Some(handle);
            }
            if self.shutdown.load(Ordering::Acquire) {
                return None;
            }
            queues = self.park.wait(queues).unwrap_or_else(poisoned);
        }
    }

    /// Signals every worker to stop after the current poll.
    pub(crate) fn set_shutdown(&self) {
        let _queues = self.queues.lock().unwrap_or_else(poisoned);
        self.shutdown.store(true, Ordering::Release);
        self.park.notify_all();
    }

    /// Whether workers should stop.
    #[must_use]
    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    /// Removes a dropped task from the registry.
    pub(crate) fn forget(&self, id: usize) {
        self.registry.lock().unwrap_or_else(poisoned).remove(&id);
    }

    /// Defers freeing a handle until no worker can reference it.
    pub(crate) fn retire(&self, handle: *mut AsyncHandle) {
        if handle.is_null() {
            return;
        }
        self.retired.lock().unwrap_or_else(poisoned).push(handle);
    }

    fn flush_retired(&self) {
        let retired: Vec<_> = std::mem::take(&mut *self.retired.lock().unwrap_or_else(poisoned));
        for handle in retired {
            if !handle.is_null() {
                // SAFETY: each retired handle is freed exactly once here, after
                // every worker has stopped and no callback can run again.
                drop(unsafe { Box::from_raw(handle) });
            }
        }
    }

    /// Drops every remaining scheduled task and frees retired handles.
    pub(crate) fn drain(&self) {
        let _run = self.run_lock.lock().unwrap_or_else(poisoned);
        let handles: Vec<_> = self
            .registry
            .lock()
            .unwrap_or_else(poisoned)
            .drain()
            .map(|(_, handle)| handle)
            .collect();
        for handle in handles {
            if !handle.is_null() {
                // SAFETY: each handle is live and dropped exactly once here.
                unsafe { crate::async_handle::vut_rt_async_drop_v1(handle) };
            }
        }
        self.flush_retired();
        // No worker is running, so any remaining queued pointers are stale.
        self.queues.lock().unwrap_or_else(poisoned).clear();
    }

    pub(crate) fn root(&self) -> *mut AsyncHandle {
        self.root.load(Ordering::Acquire)
    }

    pub(crate) fn set_root(&self, root: *mut AsyncHandle) {
        self.root.store(root, Ordering::Release);
    }

    /// Serializes a program run against other runs and `drain`.
    pub(crate) fn lock_run(&self) -> std::sync::MutexGuard<'_, ()> {
        self.run_lock.lock().unwrap_or_else(poisoned)
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
    }

    pub(crate) fn begin_run(&self) {
        // Tasks may already be scheduled (the root is spawned before the run),
        // so the queues are not cleared here; `drain` clears them after freeing.
        self.shutdown.store(false, Ordering::Release);
        self.running.store(0, Ordering::Release);
        self.max_running.store(0, Ordering::Release);
    }

    pub(crate) fn running_start(&self) -> usize {
        self.running.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub(crate) fn running_end(&self) {
        self.running.fetch_sub(1, Ordering::AcqRel);
    }

    pub(crate) fn note_max_running(&self, now: usize) {
        self.max_running.fetch_max(now, Ordering::AcqRel);
    }

    #[must_use]
    pub(crate) fn max_running(&self) -> usize {
        self.max_running.load(Ordering::Acquire)
    }

    /// Records the configured worker count for a run and caps replacements.
    pub(crate) fn configure_workers(&self, workers: usize) {
        self.desired_workers.store(workers, Ordering::Release);
        // Allow a bounded number of replacement workers while some workers are
        // blocked in native calls.
        let max = workers.saturating_add(4).max(1);
        self.max_workers.store(max, Ordering::Release);
        self.total_workers.store(0, Ordering::Release);
        self.blocked_workers.store(0, Ordering::Release);
        self.next_index.store(0, Ordering::Release);
        self.queues.lock().unwrap_or_else(poisoned).resize(max);
    }

    /// Hands out a local-queue index for a worker (wrapping within capacity).
    pub(crate) fn claim_worker_index(&self) -> usize {
        let capacity = self.max_workers.load(Ordering::Acquire).max(1);
        self.next_index.fetch_add(1, Ordering::AcqRel) % capacity
    }

    pub(crate) fn register_worker(&self) {
        self.total_workers.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn worker_exit(&self) {
        self.total_workers.fetch_sub(1, Ordering::AcqRel);
    }

    /// Retires a surplus worker if one is still surplus.
    pub(crate) fn try_retire(&self) -> bool {
        if self.blocked_workers.load(Ordering::Acquire) != 0 {
            return false;
        }
        let desired = self.desired_workers.load(Ordering::Acquire);
        self.total_workers
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |total| {
                (total > desired).then_some(total - 1)
            })
            .is_ok()
    }

    /// Registers a worker thread's join handle for the run.
    pub(crate) fn track_worker(&self, handle: std::thread::JoinHandle<()>) {
        self.workers.lock().unwrap_or_else(poisoned).push(handle);
    }

    /// Takes every worker join handle, preventing further replacements.
    pub(crate) fn take_workers(&self) -> Vec<std::thread::JoinHandle<()>> {
        std::mem::take(&mut *self.workers.lock().unwrap_or_else(poisoned))
    }

    /// Marks the current worker as blocked in a native call and, if the pool
    /// would otherwise lose parallelism, spawns a replacement worker.
    pub(crate) fn worker_blocked(&self) {
        self.blocked_workers.fetch_add(1, Ordering::AcqRel);
        let total = self.total_workers.load(Ordering::Acquire);
        let blocked = self.blocked_workers.load(Ordering::Acquire);
        let desired = self.desired_workers.load(Ordering::Acquire);
        let max = self.max_workers.load(Ordering::Acquire);
        if total.saturating_sub(blocked) < desired && total < max {
            worker::spawn_replacement(global());
        }
    }

    pub(crate) fn worker_unblocked(&self) {
        self.blocked_workers.fetch_sub(1, Ordering::AcqRel);
        self.park.notify_all();
    }
}

static SCHEDULER: OnceLock<Arc<Scheduler>> = OnceLock::new();

thread_local! {
    /// Whether the current thread is running the scheduler worker loop.
    static IN_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The current worker's local-queue index.
    static WORKER_INDEX: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

pub(crate) fn set_in_worker(value: bool) {
    IN_WORKER.with(|flag| flag.set(value));
}

/// Sets the current worker's local-queue index (`usize::MAX` clears it).
pub(crate) fn set_worker_index(index: usize) {
    WORKER_INDEX.with(|slot| slot.set(index));
}

#[must_use]
pub(crate) fn current_worker_index() -> usize {
    WORKER_INDEX.with(std::cell::Cell::get)
}

/// Whether the current thread is a scheduler worker (so a blocking call should
/// be offloaded instead of run inline).
#[must_use]
pub(crate) fn in_worker() -> bool {
    IN_WORKER.with(std::cell::Cell::get)
}

/// Returns the process-global scheduler, creating it on first use.
pub(crate) fn global() -> &'static Arc<Scheduler> {
    SCHEDULER.get_or_init(|| Arc::new(Scheduler::new()))
}

/// Returns the scheduler if it has been created (async runtime was used).
pub(crate) fn scheduler_opt() -> Option<&'static Arc<Scheduler>> {
    SCHEDULER.get()
}

/// Signals that the current worker is about to block in a native call.
pub(crate) fn on_worker_blocked() {
    if let Some(scheduler) = scheduler_opt() {
        scheduler.worker_blocked();
    }
}

/// Signals that the current worker finished blocking.
pub(crate) fn on_worker_unblocked() {
    if let Some(scheduler) = scheduler_opt() {
        scheduler.worker_unblocked();
    }
}

/// Removes a dropped task from the registry if a scheduler exists.
pub(crate) fn forget_handle(handle: *mut AsyncHandle) {
    if let Some(scheduler) = scheduler_opt()
        && let Some(id) = task::task_id(handle)
    {
        scheduler.forget(id);
    }
}

/// Defers a handle's reclamation while a run is active, otherwise frees it now.
pub(crate) fn reclaim(handle: *mut AsyncHandle) {
    if handle.is_null() {
        return;
    }
    if let Some(scheduler) = scheduler_opt()
        && scheduler.is_active()
    {
        scheduler.retire(handle);
        return;
    }
    // SAFETY: no worker is running, so the handle is owned by this call.
    drop(unsafe { Box::from_raw(handle) });
}

/// Instrumentation ABI: the maximum number of polls that ever overlapped.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_scheduler_max_running_v1() -> usize {
    scheduler_opt().map_or(0, |scheduler| scheduler.max_running())
}

/// Instrumentation ABI: the configured worker count for this process.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_scheduler_worker_count_v1() -> usize {
    configured_workers()
}
