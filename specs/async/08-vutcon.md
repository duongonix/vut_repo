# Vutcon — Lightweight Concurrent Execution Units

## 1. Purpose

This document defines `vut(...)`, `vutcon[T]`, and `await` on a `vutcon` handle.

A **Vutcon** is a lightweight concurrent execution unit, in the spirit of
goroutines / virtual threads, integrated with Vut's `async`/`await` and memory
model. A Vutcon is **not** an OS thread.

The current phase executes all Vutcons on a single OS thread with a minimal
deterministic scheduler. The design keeps the language surface fixed so a future
M:N scheduler can multiplex many Vutcons onto a small worker pool without any
source change.

This document refines `specs/async/*` and does not contradict `specs/20-concurrency.md`.

---

## 2. Surface Syntax

Canonical one-line:

```vut
job = vut(() => calculate() * 2)
```

Canonical multiline:

```vut
job = vut(fn():
  value = calculate()
  value * 2
)
```

Awaiting:

```vut
result = await job
```

Typed handle:

```vut
job: vutcon[int] = vut(() => calculate())
```

Result-typed handle and propagation:

```vut
job: vutcon[result[bytes, AppError]] = vut(() => load())
data = await job?
```

Rules:

```text
vut(<callable>)           only form; the argument is an anonymous callable
callable                  `() => expression` or `fn():`-block, no parameters
vutcon[T]                 handle type; `T` is the logical result type
await handle              suspends until the Vutcon completes; yields `T`
await handle?             allowed when `T` is `result[_, E]`
```

Not part of the language:

```text
vut calculate()
vut calculate
vut fn():
  ...
```

`vut` is a reserved keyword. `vutcon` is a builtin parameterized type name
(like `list`, `map`, `result`).

---

## 3. Semantics

```text
vut(callback)   create and schedule a Vutcon; result: vutcon[T]
vutcon[T]       typed handle for a Vutcon whose logical result is T
await handle    suspend until completion; yields T
```

`vut(...)` always produces a `vutcon[T]`, including when the handle is
discarded. There is no separate fire-and-forget semantics.

Callbacks take **no parameters** in this phase. Unlike general synchronous
lambdas, which may capture (`specs/04-functions-methods.md` §46a), Vutcon
callbacks remain **non-capturing** in this phase; captured values and an
environment pointer for Vutcon callbacks are reserved for a later phase.

A callback may be a plain `fn()` (synchronous) or an anonymous `async fn()`
(suspendable). Both produce `vutcon[T]`, where `T` is the callable's logical
result type. A plain `fn()` body runs on its first poll and yields `Ready(T)`
immediately; an `async fn()` body may poll to `Pending`, wake, and resume. Only
`async fn()` may contain `await`.

`vut(...)` is only valid inside an `async fn`/`async method`, where an executor
context exists.

---

## 4. Relation to `async`/`await`

```text
async fn    a function that may suspend
await       suspension/wait mechanism
vut         spawn/schedule a lightweight concurrent unit
vutcon[T]   typed handle of that unit
```

`vut` does not replace `async`. An async function can run inside a Vutcon:

```vut
job = vut(() => fetch())
data = await job
```

`await` accepts two operand forms:

```text
await async_call       (existing; drives a direct async computation)
await vutcon_handle    (new; suspends until the Vutcon completes)
```

`await` and `?` remain independent. `await handle?` parses as
`(await handle)?`.

---

## 5. Execution and Scheduling

Current phase (M2.2):

```text
many Vutcons
     ↓
shared scheduler (ready set)
     ↓
M worker OS threads (caller = worker 0)
     ↓
multiple CPU cores
```

`vut(callable)` creates a task and **schedules it immediately**: the task is
enqueued in the scheduler ready set before `vut(...)` returns. `await job` only
waits for the task's completion; it is not the operation that starts the task.

`M` is `VUT_MAXPROCS` when set to `>= 1`, otherwise `available_parallelism()`;
it is never zero. Workers use per-worker local queues plus a global injector and
steal from each other, so two Vutcons may run **in parallel** on different
cores. Blocking native calls are offloaded to a blocking pool. A task never
yields except at `await`; there is no preemption.

```text
task poll -> Pending    register wake
task poll -> Ready(T)   store result, drop the task when awaited/dropped
```

Because a scheduled task may already be running, the language does **not**
guarantee that an unawaited handle's body never runs. How far a task progresses
before its handle is dropped is scheduler-dependent; dropping an unawaited
handle destroys the task safely (cancelling it if it is suspended) but does not
guarantee the body never started.

`await` suspends the awaiting task until the awaited task completes, then yields
its result. Awaiting a task that is already complete returns immediately.

A Vutcon body may itself await other Vutcons or native futures; the scheduler
resumes it when the awaited work wakes.

Still not implemented in this phase: a reactor, preemption, per-task memory
reclamation (task memory is freed at shutdown), channels, mutex/rwlock, and an
atomic API.

---

## 5a. Scheduler Invariants

```text
a task is queued at most once (`queued` guard)
one worker polls a task at a time (`running` guard)
a drop during a poll is finished by that worker (exactly-once destructor)
no handle is freed while a worker may reference it (deferred reclamation)
wake never loses a signal (queued set under the ready lock; completion flags)
shutdown is signalled under the ready lock, then workers join before cleanup
```

---

## 6. Memory and Ownership

Vutcons obey the existing memory model: value semantics, compiler-managed
ownership, automatic moves, deterministic drop, no tracing GC.

```text
handle                        managed runtime value (one owner)
result                        owned by the completed task until awaited
await handle                  consumes the handle; moves the result out
dropped unawaited handle      destroys the task safely (cancels if suspended)
```

`vut(...)` schedules the task immediately, so an unawaited handle may already
have started; dropping destroys it safely but does not promise the body never
ran (see §5). `await` consumes the handle and moves the result out. Dropping
after `await` is impossible because `await` consumed the handle.

The compiler must insert cleanup for live handles on every exit path (normal
completion, early `return`, `?` propagation).

---

## 7. Compiler and Runtime Boundary

Language/compiler surface: `vut`, `vutcon[T]`, `await`, `await ...?`.

Runtime contract (opaque, C ABI):

```text
spawn_sync(start_fn, size, align)         -> opaque task handle  (immediate Ready)
spawn_async(frame, poll_fn, drop_fn)      -> opaque task handle  (poll/resume)
poll(handle, out)                         -> Pending | Ready     (drives the task)
drop(handle)                              -> destroy/cancel the task
```

Both construction entry points converge on **one poll-task representation**:
after construction a task is `(state, poll_fn, drop_fn)` and is driven by
`poll`/`wake`, whether its body is a plain `fn()` (first poll yields `Ready`) or
an `async fn()` (may yield `Pending`, `wake`, resume). There is no second async
runtime for Vutcons.

The executor is the same generic single-thread executor used by `async fn` and
native futures: it owns the ready set and the poll/wake seam, enqueues scheduled
tasks, resumes them on wake, and drives the root task to completion. No Rust
`Future`/`Waker`/`Poll`/trait object crosses the C ABI
(`specs/async/04-runtime.md` §9).

The scheduler is replaceable behind this ABI; `vut(...)`, `vutcon[T]`, and
`await` do not change for M:N.

---

## 8. Diagnostics

```text
vut(...) operand is not an anonymous callable
callback declares parameters
invalid capture in callback
vut(...) outside an async body
await on a non-vutcon / non-async operand
vutcon[T] arity or unknown inner type
handle used after await
await handle? where T is not result[_, E]
unsupported Vutcon result type
internal scheduler/lowering failure
```

---

## 9. Principles

1. A Vutcon is not an OS thread.
2. `vut` spawns; `await` waits; `vutcon[T]` is the typed handle.
3. Vutcon callbacks are non-capturing and parameterless in this phase.
4. `vut` is only valid in an async body.
5. `await` consumes the handle and yields `T`.
6. Dropping an unawaited handle destroys the Vutcon.
7. Value semantics and deterministic drop are preserved.
8. The scheduler is M:N: many Vutcons on `M` worker threads.
9. No work-stealing, blocking offload, reactor, or preemption is exposed.
10. No Rust runtime concepts appear in Vut source.
