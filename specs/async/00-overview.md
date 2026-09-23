# Async Overview

## 1. Purpose

This specification defines `async fn` and `await` for Vut.

It also defines the minimum single-threaded execution model required to make
those two mechanisms work correctly.

The goal of this phase is a correct language foundation, not a concurrency
framework.

---

## 2. Scope

This specification covers:

```text
async fn
await
async function typing
await typing
async lowering / state machines
single-thread executor foundation
ownership and destruction across suspension
async diagnostics
async main
Result interaction
```

Canonical example:

```vut
async fn load() -> int:
  value = await get_value()
  value
```

Calling an `async fn` produces an awaitable value. `await` retrieves the logical
result of that computation.

---

## 3. Explicit Non-Goals

The following are intentionally out of scope for this phase:

```text
threads
worker pools
multi-thread schedulers
parallel execution
channels
mutex
rwlock
atomic public API
structured concurrency
task.spawn
task groups
async networking
async filesystem
select
join
race
timeout frameworks
```

These may be specified later. They must not be implemented or implied by this
phase.

---

## 4. `async` Is Not Parallelism

This phase is single-threaded.

```text
async != parallel
async != multithreading
```

Programs using `async`/`await` execute:

```text
one OS thread
zero or more async state machines
no parallel execution
```

If no spawn/concurrent-task API exists yet, a chain of async computations runs
only through the dependency expressed by `await`.

The compiler and runtime may not run async computations concurrently merely as
an optimization.

---

## 5. Current Phase vs Future Concurrency

This specification supersedes the "async is deferred" statements for the
`async fn` + `await` subset only.

```text
Current phase (this specification)
  async fn
  await
  single-thread state-machine foundation
  single-thread executor
  async main

Future concurrency phases (still deferred)
  native threads
  task.spawn / structured concurrency
  channels
  synchronization primitives
  multi-thread scheduler
  async I/O and reactors
```

The long-term concurrency direction remains governed by
`specs/20-concurrency.md`. This document only finalizes the single-thread
foundation.

---

## 6. User-Facing Model

The programmer writes logical signatures:

```vut
async fn fetch() -> Data:
  ...
```

without writing future/promise types:

```text
Future(Data)
Promise(Data)
Poll(Data)
Waker
Context
Pin
```

These concepts may exist internally in the compiler/runtime. They are not
source-level Vut types in this phase.

---

## 7. Execution Model Summary

```text
call async fn
    ↓
create awaitable state machine (lazy)
    ↓
await drives it
    ↓
runs to completion on the current single thread
    ↓
logical result is produced
```

This phase supports suspension points in the model even though the initial
runtime has no asynchronous I/O source yet.

---

## 8. Relationship to Other Specifications

```text
specs/01-language-syntax.md      keyword and surface syntax
specs/02-type-system.md          type-system integration
specs/04-functions-methods.md    function and method model
specs/08-memory-model.md         ownership across suspension
specs/10-compiler-architecture.md pipeline integration
specs/11-runtime.md              runtime integration
specs/20-concurrency.md          long-term concurrency direction
specs/21-grammar.md              formal grammar
specs/22-error-codes.md          diagnostic codes
specs/compiler/*.md              stage-level representation
```

This directory refines those documents for the async subset.

---

## 9. Principles

1. `async fn` has a logical signature; the call result is an internal awaitable.
2. `await` is a prefix expression with a single canonical syntax.
3. `await` only exists inside `async fn`.
4. Async is value-semantic and deterministic; it does not require a tracing GC.
5. Suspension preserves Vut's automatic move and deterministic drop rules.
6. Async does not require multiple threads.
7. Unused async features must not add runtime cost to ordinary programs.
8. Future concurrency is not implemented by this phase.

---

## 10. Implementation Status

The single-thread foundation is implemented.

Because this phase has no asynchronous primitive, no `await` can ever suspend.
The compiler therefore lowers an `async fn` and its `await` sites directly:

```text
await async_call(args)
    ↓
async_call(args)   (the body runs to completion on the current thread)
```

Consequences for the implemented subset:

```text
await operand must be an async call (possibly parenthesized)
awaitable values are not first-class; they cannot be stored or passed
the single-thread executor is not required yet
ownership across `await` is ordinary sequential ownership
```

The state-machine model in `03-lowering.md` and the executor in `04-runtime.md`
remain the normative design that must be used once a real suspension source
(asynchronous I/O or an explicit suspension primitive) is introduced. While no
await can suspend, the direct lowering above is observationally equivalent to
that model.

Ordinary programs that do not use async pay no async runtime cost.

---

## Vutcon extension

`specs/async/08-vutcon.md` extends this subset with lightweight concurrent
execution units:

```text
vut(<callable>)      spawn and schedule a Vutcon
vutcon[T]             typed handle for the Vutcon's logical result
await handle          suspend until the Vutcon completes; yields T
```

Vutcons run on the single-thread scheduler in the current phase and are
designed so a future M:N worker scheduler can replace the scheduler internals
without changing the surface syntax.
