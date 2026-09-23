# M2.2 — Vutcon Production Runtime

## Status

Complete for Phases 0–4. Still deferred: async I/O reactor (epoll/kqueue/IOCP),
per-task memory reclamation, public channel/mutex/atomic API, preemption, and
stack growth.

## Goal

Turn the single-thread poll executor into a production M:N scheduler that
multiplexes many Vutcons onto `M` worker OS threads with parallel execution on
multiple cores, without changing the language surface or the compiler ABI.

## Phase 0 — shared scheduler, one worker

- Replaced the thread-local executor with a process-global `Scheduler`
  (`crates/vut-runtime/src/scheduler/`).
- Removed the boxed `Resume` context; the resume callback carries the task handle.
- `CURRENT` remains a worker-local thread-local used only while polling.
- Observable semantics unchanged with one worker.

## Phase 1 — M:N worker pool

- `run` starts `M-1` worker threads; the caller is worker 0.
- `M = VUT_MAXPROCS` when `>= 1`, else `available_parallelism()`, never zero.
- Ready tasks run in parallel; no work stealing, blocking offload, or preemption.
- The scheduler is process-global and serializes `run`/`drain`.

## Phase 2 — parallel safety and blocking

- Audited managed values: reference counts are atomic (`AtomicUsize`) across
  `str`, `bytes`, `list`, `map`, `interface`, `closure`; internal `VutList`/
  `VutMap`/`VutString` use `Arc` copy-on-write. Cross-task sharing is not
  reachable through the language (Vutcon callbacks are parameterless and cannot
  capture), so single ownership holds across workers.
- Added a blocking pool (`crates/vut-runtime/src/blocking.rs`) with
  `blocking::run`. From a worker it offloads to a pool thread and waits;
  outside a worker it runs inline.
- The scheduler starts a bounded replacement worker while a worker is blocked,
  so the requested worker count stays available.
- Offloaded the blocking filesystem operations (`fs.open/read/read_str/write/
  write_str/append/create_dir/remove_*/copy/rename`) through `blocking::run`.
- No public concurrency API was added.

## Phase 3 — production scheduler

- Ready work is a per-worker local queue plus a global injector. Workers pop
  their local queue LIFO, drain the injector FIFO, and steal from other workers.
- Wake raised while polling enters the worker's local queue; other wakes enter
  the injector.
- Panic isolation: an unexpected panic during a poll is caught; the worker
  survives and a panicking root stops the run for cleanup. (Vut's own panic path
  aborts the process by design.)
- Shutdown hardening: shutdown is signalled under the queue lock, workers are
  joined (including replacements) before drain, and the queues are cleared after
  drain so no stale pointer survives into the next run.

## Phase 4 — optimization and hardening

- Queue clear on drain removes cross-run stale pointers (fixed heap corruption
  under repeated runs).
- Stress tests: mass spawn/drain (1000 tasks), concurrent drop/cancel during a
  run (500 tasks), repeated runs (50), plus the parallel barrier tests.
- Ignored benchmark (`benchmark_worker_scaling`) measures 1/2/4/8 workers with a
  synchronization barrier and the runtime `max_running` counter; observed
  `max_running` scales 1→2→4.

## Concurrency invariants

```text
queued     a task has at most one ready entry (wake coalescing)
running    one worker polls a task at a time
dropped    drop during a poll is finished by that worker (exactly once)
reclaim    handles are not freed while a worker may reference them
wake       queued set before enqueue; completion flags prevent loss
shutdown   signalled under the queue lock, then join workers, then drain+clear
```

## Instrumentation

- `vut_rt_scheduler_max_running_v1()` — peak overlapping polls.
- `vut_rt_scheduler_worker_count_v1()` — configured worker count.

## Known limitations

- Task memory is reclaimed only at shutdown (deferred reclamation).
- Blocking offload covers filesystem operations; process/stdio still block the
  worker that calls them.
- No reactor: async networking stays on the native HTTP Tokio runtime.
- No user-visible synchronization primitives.
- Per-worker queues share one scheduler lock; sharding is a future optimization.

## Related

- `specs/async/04-runtime.md` §3, §7, §10, §12
- `specs/async/08-vutcon.md` §5, §5a, §9
- `specs/20-concurrency.md` §22
- `specs/11-runtime.md`
