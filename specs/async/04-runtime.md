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

## 3. Single-Thread Executor

The executor owns:

```text
a ready queue of resumable futures
the currently running future
a wake mechanism
```

Conceptually:

```text
loop:
  pop next ready future
  resume it once
  if Pending, leave it suspended
  if Ready(value), complete it and wake its parent
  stop when the root future completes
```

The executor never starts worker threads and never runs two futures in parallel.

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

## 7. No Threads

This phase must not:

```text
spawn OS threads
spawn worker threads
use a thread pool
run futures in parallel
require synchronization primitives between futures
```

Runtime structures may be designed so a future multi-thread extension is
possible, but no threading is implemented here.

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
backend) and then **signal/wake** the executor; the executor itself never spawns
threads and never runs Vut tasks in parallel. A wake from another thread only
enqueues the task and notifies the executor (no busy-wait).

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
thread pools
work stealing
```

They must be defined by later specifications.

---

## 11. Zero Cost

Programs that do not use `async` must not pay for the async executor.

The async runtime is linked/emitted only when async is used.

---

## 12. Runtime Principles

1. Execution is single-threaded.
2. The executor is minimal and deterministic.
3. The wake mechanism is internal.
4. `async main` is driven automatically by the runtime.
5. No threads, pools, or parallel execution exist in this phase.
6. The native boundary remains a stable C ABI.
7. Async runtime cost is paid only by programs that use async.
