# Async Runtime

## 1. Purpose

This document defines the minimum single-threaded runtime foundation required to
execute `async fn` and `await`.

It does not define threads, parallelism, channels, or async I/O.

---

## 2. Model

```text
async state machine
+
single-thread executor
+
minimal wake/resume mechanism
```

Everything executes on the current OS thread.

### 2.1 Implementation Status

The single-thread executor below is implemented. Suspension sources exist:

```text
- native `extern "C" async fn` operations (e.g. network I/O)
- `vut(async fn(): ...)` Vutcons awaiting other work
```

`async main` is driven by the executor: the generated entry point creates the
root future for `main`, runs the executor until the root completes, and returns
the logical result. `await` suspends the current task and resumes it when the
awaited work wakes; asynchronous native work runs off the Vut thread and only
signals/wakes the executor. The executor never busy-waits.

---

## 3. Scheduler

The scheduler owns:

```text
a ready queue of resumable tasks (shared across workers)
the worker pool that polls ready tasks
a wake mechanism
```

Conceptually each worker runs:

```text
loop:
  pop next ready task
  poll it once
  if Pending, leave it suspended
  if Ready(value), complete it and wake its awaiter
  stop when the root task completes
```

M2.2 replaces the earlier single-thread executor with an M:N scheduler: many
tasks are multiplexed onto `M` worker OS threads, where the caller thread is
worker 0 and `M` is `VUT_MAXPROCS` (default `available_parallelism()`, never
zero). Tasks are polled by whichever worker pops them, so ready tasks can run in
parallel on multiple cores.

Each worker has a local ready queue and the scheduler has a global injector;
workers pop their local queue last-in-first-out, drain the injector
first-in-first-out, and steal from other workers' local queues. There is no
preemption; a task only yields at `await`.

Blocking native calls (filesystem, process, stdio) are offloaded to a blocking
pool through the runtime's `blocking::run` helper. While a worker waits on a
blocking call the scheduler may start a bounded replacement worker, so the
requested number of workers stays available.

The wake seam is thread-safe: a native completion on any thread enqueues the
task and wakes a parked worker. No scheduler lock is held across a poll, so a
polled task may spawn, wake, or drop tasks re-entrantly.

The scheduler is process-global; a single program run drives it at a time.

---

## 4. Poll Result

Resuming a future produces:

```text
Ready(value)
Pending
```

In this phase, with no asynchronous I/O source, an await is generally ready on
the first resume, and the executor may run it inline.

`Pending` exists so the model supports future asynchronous primitives without
changing the language semantics.

---

## 5. Wake/Resume

A suspended future registers a wake callback.

```text
future suspends
    ↓
callback stored
    ↓
completion (or external event) enqueues future as ready
    ↓
executor resumes it
```

The wake mechanism is single-threaded and must not race.

No public `Waker`, `Context`, or `Poll` type is exposed to Vut source.

---

## 6. `async main`

When `main` is declared `async`, the generated entry point:

```text
creates the root future for main
runs the single-thread executor until the root future completes
returns the logical result to the process
```

Return handling matches synchronous `main`: logical `void` or `int`.

A failure signaled through the program's Result/exit policy behaves like the
synchronous entry point.

The user never calls an executor API.

---

## 7. Worker Pool (M:N)

M2.2 multiplexes tasks onto `M` worker OS threads:

```text
spawn M-1 worker threads; the caller thread is worker 0
M = VUT_MAXPROCS, else available_parallelism(), never zero
ready tasks may run in parallel on multiple cores
```

Constraints that still hold:

```text
no reactor / async I/O integration
no preemption; a task yields only at `await`
no user-visible synchronization primitives (channels, mutex, atomics)
task memory is reclaimed at shutdown, not per task
```

`VUT_MAXPROCS` values below `1` or invalid fall back to the default, so a run
always has at least one worker. The scheduler is process-global and serializes
`run`/`drain`, so one program run drives it at a time.

Workers use per-worker local queues plus a global injector, and steal from each
other. Blocking native calls are offloaded to a blocking pool; a bounded
replacement worker keeps the pool's parallelism while a worker is blocked.

---

## 8. Runtime ABI

The async runtime objects are compiler/runtime internal.

The runtime ABI version must be incremented when the async object layout or
resume contract changes, consistent with:

```text
specs/23-compatibility-versioning.md
specs/compiler/codegen.md
```

The compiler must reject a mismatched async runtime ABI.

---

## 9. Native Async Boundary

If native code later needs to produce or drive async computations, the boundary
must remain a stable C ABI.

The following must never cross the C ABI directly:

```text
Rust Future
Rust trait object
Rust async ABI
Rust Waker/Context/Poll types
```

A future native async extension point should be a C-compatible contract,
conceptually:

```text
vut_async_start(...)   -> opaque handle
vut_async_poll(handle) -> status
vut_async_drop(handle)
```

Native libraries may perform work on their own threads (for example an I/O
backend) and then **signal/wake** the scheduler; the wake enqueues the task and
wakes a parked worker (no busy-wait). Native work is never executed by a
scheduler worker itself.

The exact symbols and struct layouts are not finalized in this phase.

Because this phase has no real asynchronous native source, no native async
primitive is required. The extension point is documented only.

---

## 10. Deferred Runtime Features

Still deferred:

```text
timers
event loops / reactors
epoll / kqueue / IOCP / io_uring integration
async filesystem
async networking
preemption
per-task reclamation (task memory is freed at shutdown)
```

They must be defined by later specifications.

---

## 11. Zero Cost

Programs that do not use `async` must not pay for the async executor.

The async runtime is linked/emitted only when async is used.

---

## 12. Runtime Principles

1. Execution is M:N: many tasks on `M` worker threads.
2. The scheduler is minimal and deterministic for a given worker count.
3. The wake mechanism is internal and thread-safe.
4. `async main` is driven automatically by the runtime.
5. Workers steal work; blocking calls are offloaded to a blocking pool.
6. The native boundary remains a stable C ABI.
7. Async runtime cost is paid only by programs that use async.
