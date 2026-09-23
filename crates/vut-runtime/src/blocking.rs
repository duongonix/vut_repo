//! Blocking offload pool.
//!
//! Native calls that can block the calling OS thread (filesystem, process,
//! stdio) are routed through [`run`]. When called from a scheduler worker, the
//! work runs on a pool thread while the worker waits, and the scheduler is told
//! a worker is blocked so it can keep the requested number of workers active.
//! Outside a worker the closure runs inline, so tests and single-threaded
//! callers are unaffected.
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Pool {
    queue: Mutex<VecDeque<Job>>,
    available: Condvar,
    threads: AtomicUsize,
    max_threads: usize,
}

impl Pool {
    fn new(max_threads: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            available: Condvar::new(),
            threads: AtomicUsize::new(0),
            max_threads,
        }
    }

    /// Enqueues a job, spawning a pool thread if there is capacity. Returns the
    /// job back when no thread can be started, so the caller can run it inline.
    fn submit(self: &Arc<Self>, job: Job) -> Result<(), Job> {
        {
            let mut queue = self.queue.lock().unwrap_or_else(poison);
            queue.push_back(job);
        }
        if self.ensure_thread() {
            self.available.notify_one();
            Ok(())
        } else {
            // No thread and none can be started: take the job back and run it.
            let job = self.queue.lock().unwrap_or_else(poison).pop_back();
            Err(job.unwrap_or_else(|| Box::new(|| {})))
        }
    }

    fn ensure_thread(self: &Arc<Self>) -> bool {
        loop {
            let current = self.threads.load(Ordering::Acquire);
            if current >= self.max_threads {
                // Threads already exist and will drain the queue.
                return current > 0;
            }
            if self
                .threads
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            let pool = Arc::clone(self);
            let spawned = std::thread::Builder::new()
                .name("vut-blocking".to_owned())
                .spawn(move || pool.worker())
                .is_ok();
            if spawned {
                return true;
            }
            self.threads.fetch_sub(1, Ordering::AcqRel);
            return self.threads.load(Ordering::Acquire) > 0;
        }
    }

    fn worker(self: Arc<Self>) {
        loop {
            let job = {
                let mut queue = self.queue.lock().unwrap_or_else(poison);
                loop {
                    if let Some(job) = queue.pop_front() {
                        break job;
                    }
                    queue = self.available.wait(queue).unwrap_or_else(poison);
                }
            };
            // A panicking job must not take down the pool thread.
            let _ = std::panic::catch_unwind(AssertUnwindSafe(job));
        }
    }
}

fn poison<T>(error: std::sync::PoisonError<T>) -> T {
    error.into_inner()
}

fn pool() -> &'static Arc<Pool> {
    static POOL: OnceLock<Arc<Pool>> = OnceLock::new();
    POOL.get_or_init(|| Arc::new(Pool::new(default_threads())))
}

fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(4, std::num::NonZeroUsize::get)
        .clamp(2, 16)
}

/// Runs `work`, offloading to the blocking pool when called from a scheduler
/// worker. The closure must own its data (`'static`).
pub fn run<R, F>(work: F) -> R
where
    R: Send + 'static,
    F: FnOnce() -> R + Send + 'static,
{
    if !crate::scheduler::in_worker() {
        return work();
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel::<R>(1);
    let job: Job = Box::new(move || {
        let _ = sender.send(work());
    });
    crate::scheduler::on_worker_blocked();
    let outcome = match pool().submit(job) {
        Ok(()) => receiver.recv(),
        // No pool thread could be started; run inline so we never deadlock.
        Err(job) => {
            job();
            receiver.recv()
        }
    };
    crate::scheduler::on_worker_unblocked();
    match outcome {
        Ok(value) => value,
        Err(_) => std::panic::resume_unwind(Box::new("blocking job failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn offloads_to_another_thread_from_a_worker() {
        crate::scheduler::set_in_worker(true);
        let caller = std::thread::current().id();
        let ran_on = run(|| std::thread::current().id());
        crate::scheduler::set_in_worker(false);
        assert_ne!(ran_on, caller, "a worker must offload blocking work");
    }

    #[test]
    fn runs_inline_outside_a_worker() {
        crate::scheduler::set_in_worker(false);
        let caller = std::thread::current().id();
        let ran_on = run(|| std::thread::current().id());
        assert_eq!(ran_on, caller, "outside a worker the work runs inline");
    }
}
