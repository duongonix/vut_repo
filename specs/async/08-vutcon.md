# Vutcon — Lightweight Concurrent Execution Units

## 1. Purpose

This document defines `vut(...)`, `vutcon(T)`, and `await` on a `vutcon` handle.

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
job: vutcon(int) = vut(() => calculate())
```

Result-typed handle and propagation:

```vut
job: vutcon(result(bytes, AppError)) = vut(() => load())
data = await job?
```

Rules:

```text
vut(<callable>)           only form; the argument is an anonymous callable
callable                  `() => expression` or `fn():`-block, no parameters
vutcon(T)                 handle type; `T` is the logical result type
await handle              suspends until the Vutcon completes; yields `T`
await handle?             allowed when `T` is `result(_, E)`
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
vut(callback)   create and schedule a Vutcon; result: vutcon(T)
vutcon(T)       typed handle for a Vutcon whose logical result is T
await handle    suspend until completion; yields T
```

`vut(...)` always produces a `vutcon(T)`, including when the handle is
discarded. There is no separate fire-and-forget semantics.

Callbacks take **no parameters** in this phase. Callbacks are **non-capturing**
in this phase (consistent with `specs/04-functions-methods.md` §46); captured
values and an environment pointer are reserved for a later phase.

A callback may be a plain `fn()` (synchronous) or an anonymous `async fn()`
(suspendable). Both produce `vutcon(T)`, where `T` is the callable's logical
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
vutcon(T)   typed handle of that unit
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

Current phase:

```text
many Vutcons
     ↓
single-thread scheduler (ready set)
     ↓
one OS thread
```

`vut(callable)` creates a task and **schedules it immediately**: the task is
enqueued in the executor ready set before `vut(...)` returns. `await job` only
waits for the task's completion; it is not the operation that starts the task.

The executor runs ready tasks when the currently running task suspends. It never
preempts and never runs two tasks in parallel:

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

A Vutcon body may itself await other Vutcons or native futures; the executor
resumes it when the awaited work wakes, so multiple Vutcons can make progress
concurrently (interleaved on one thread). No two Vutcons execute in parallel.

Future phase (no surface change):

```text
many Vutcons
     ↓
Vut scheduler
     ↓
small worker pool
     ↓
OS threads
```

Not implemented in this phase: OS thread API, worker pool, multithread
scheduler, parallel guarantee, channels, mutex/rwlock, atomic API, work
stealing.

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

Language/compiler surface: `vut`, `vutcon(T)`, `await`, `await ...?`.

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

The scheduler is replaceable behind this ABI; `vut(...)`, `vutcon(T)`, and
`await` do not change for M:N.

---

## 8. Diagnostics

```text
vut(...) operand is not an anonymous callable
callback declares parameters
invalid capture in callback
vut(...) outside an async body
await on a non-vutcon / non-async operand
vutcon(T) arity or unknown inner type
handle used after await
await handle? where T is not result(_, E)
unsupported Vutcon result type
internal scheduler/lowering failure
```

---

## 9. Principles

1. A Vutcon is not an OS thread.
2. `vut` spawns; `await` waits; `vutcon(T)` is the typed handle.
3. Callbacks are non-capturing and parameterless in this phase.
4. `vut` is only valid in an async body.
5. `await` consumes the handle and yields `T`.
6. Dropping an unawaited handle destroys the Vutcon.
7. Value semantics and deterministic drop are preserved.
8. The scheduler is single-threaded now and replaceable for M:N later.
9. No parallel-execution guarantee is exposed.
10. No Rust runtime concepts appear in Vut source.
