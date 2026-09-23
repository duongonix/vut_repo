# Vut Concurrency

## 1. Purpose

This document defines the current direction and boundaries for concurrency in Vut.

Concurrency is not required for the initial Vut MVP.

The first compiler implementation should not delay core language completion in order to implement advanced concurrency.

The architecture should nevertheless avoid decisions that make safe concurrency impossible later.

---

## 2. Phase Status

The single-thread `async`/`await` foundation is now specified and implemented by
`specs/async/`:

```text
async fn             implemented (single-thread)
await                implemented (single-thread)
single-thread executor implemented (drives async main and Vutcons)
vut / vutcon[T]      implemented (schedule concurrent tasks; sync + async callables)
threads API          deferred
channels             deferred
actors               deferred
atomics API          deferred
structured concurrency deferred
multi-thread scheduler deferred
```

Async/await in this phase is single-threaded and must not be read as a request
to implement full concurrency.

The remaining concurrency features must not be invented by the compiler before
their syntax and semantics are specified.

See:

```text
specs/async/00-overview.md
```

for the boundary between the current async phase and future concurrency phases.

---

## 3. Design Goals

Future Vut concurrency should aim for:

* memory safety
* predictable behavior
* native performance
* minimal runtime overhead
* cross-platform support
* structured lifetime of tasks
* clear cancellation behavior
* strong static typing

---

## 4. No Implicit Concurrency

Normal Vut code executes according to normal sequential semantics unless concurrency is explicitly requested.

Example:

```vut
a()
b()
c()
```

must not run those functions concurrently merely as an optimization.

The compiler may internally parallelize compilation, but that is unrelated to program semantics.

---

## 5. Runtime Independence

The core Vut runtime must not require a heavyweight async executor merely because concurrency may be added later.

Async programs use only the minimal single-thread executor defined in
`specs/async/04-runtime.md`.

Programs not using concurrency or async should not pay large scheduler/runtime costs.

---

## 6. Native Threads

Future concurrency may expose native threads through the standard library.

Conceptually:

```text
std.thread
```

Exact API and syntax are not yet defined.

Do not introduce a `thread` keyword unless explicitly specified later.

---

## 7. Async/Await

`async fn` and `await` are specified for a single-threaded execution model.

The locked surface syntax is:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

`await` is a prefix expression and is only valid inside an `async fn`.

Detailed behavior belongs to `specs/async/`.

This does not introduce:

```text
task.spawn
worker threads
parallel execution
channels
mutex
```

Async in this phase means one OS thread with suspend/resume state machines.

The parser and compiler must implement only the specified single-thread subset.

---

## 8. Structured Concurrency

If asynchronous tasks are added, structured concurrency should be preferred over unbounded detached task creation.

The goal is for task lifetime to be understandable from program structure.

Detailed semantics require a future specification.

---

## 9. Detached Tasks

Detached/background tasks should not be the default concurrency primitive.

If supported later, their:

* lifetime
* error handling
* process shutdown behavior
* resource ownership

must be explicit.

---

## 10. Memory Safety

Safe concurrent Vut code must not introduce memory unsafety.

The language/runtime must prevent safe code from causing:

* use-after-free
* double-free
* invalid shared-memory access

Data race policy must be defined before shared mutable concurrency is stabilized.

---

## 11. Data Races

The final language should aim to prevent or strongly control unsynchronized data races in safe code.

However, the exact static/runtime model is not yet locked.

Possible implementation strategies must not be exposed prematurely as language syntax.

---

## 12. Value Semantics and Concurrency

Vut's value-oriented semantics may simplify concurrency because copied/logically independent values do not automatically share mutable state.

Example:

```vut
a = value
b = a
```

must not unexpectedly create shared mutable aliasing merely because a runtime optimization shares storage internally.

The compiler/runtime must preserve this rule under concurrency.

---

## 13. Copy-on-Write

If collections use copy-on-write internally, the implementation must remain thread-safe when values cross concurrency boundaries.

This is an implementation concern.

Source-level value semantics remain unchanged.

---

## 14. Shared State

Future explicit shared-state abstractions must define synchronization behavior clearly.

Potential standard-library mechanisms may include:

```text
mutex
rwlock
atomic
```

Exact names/APIs are deferred.

---

## 15. Atomic Types

Atomic operations require precise memory-ordering semantics.

Do not expose atomics until the standard library/runtime specifies:

* supported atomic types
* ordering modes
* compare/exchange behavior
* platform guarantees

---

## 16. Channels

Typed channels are a possible future communication mechanism.

Conceptually:

```text
channel(T)
```

but this syntax is not locked.

If introduced, channels should remain statically typed.

Do not use `dyn` as the default message type.

---

## 17. Task Errors

Concurrent task failures should integrate with Vut's Result-style error philosophy.

Expected failures should not require exception handling.

The exact task result API is deferred.

---

## 18. Cancellation

Async/task cancellation must have defined semantics before async execution becomes stable.

For the single-thread phase, dropping an incomplete future destroys it safely
and drops its live state exactly once. This is specified in:

```text
specs/async/05-memory.md
```

A user-facing cancellation API is not part of this phase.

Important concerns for future cancellation include:

* cleanup
* partial operations
* resource release
* cancellation propagation

Cancellation must not simply terminate arbitrary code at unsafe execution points.

---

## 19. Resource Safety

Concurrent code must preserve resource-management guarantees.

Examples:

```text
file handles
locks
sockets
tasks
```

A future structured-resource system should integrate with concurrency rather than rely on unpredictable memory cleanup.

---

## 20. Interface Dispatch

Interfaces must remain usable in concurrent programs where underlying values satisfy required thread-safety guarantees.

The exact marker/interface model for thread safety is not yet defined.

Do not automatically add Rust concepts such as:

```text
Send
Sync
```

to Vut syntax without a separate design decision.

---

## 21. `dyn` in Concurrent Code

Dynamic values may require additional runtime synchronization when shared.

The existence of `dyn` must not force synchronization overhead on ordinary statically typed values.

---

## 22. Runtime Scheduler

The scheduler is M:N: many tasks are multiplexed onto `M` worker OS threads
(the caller is worker 0; `M` is `VUT_MAXPROCS` or `available_parallelism()`).
See:

```text
specs/async/04-runtime.md
specs/async/08-vutcon.md
```

Scheduler internals are an implementation detail unless observable behavior
depends on them.

Still deferred:

```text
work stealing
blocking offload
platform event loops / reactors
preemption
```

Programs that do not use async do not require a scheduler.

---

## 23. Existing Runtime Libraries

If async/concurrency is implemented, the project should evaluate proven Rust libraries instead of writing every scheduler, reactor or platform I/O implementation from scratch.

However, exposing a Rust library internally must not force Rust-specific concepts into Vut's public language design.

---

## 24. OS Async APIs

Future runtime implementations may use platform mechanisms such as:

```text
IOCP
epoll
kqueue
io_uring
```

where appropriate.

These are runtime implementation details.

Vut source should use portable APIs unless explicitly requesting platform-specific behavior.

---

## 25. Parallel Collections

Parallel iteration is not part of the core `for` semantics.

This:

```vut
for item in items:
  process(item)
```

is sequential unless a future explicitly parallel API is used.

Do not automatically parallelize loops if it could change observable behavior.

---

## 26. Compiler Parallelism

The Vut compiler itself may compile independent modules in parallel.

This does not require any Vut language concurrency syntax.

Compiler parallelism must preserve deterministic diagnostics and build results.

---

## 27. Thread-Local State

Future thread-local storage requires an explicit library/language contract.

Do not infer that ordinary globals become thread-local automatically.

---

## 28. Global State

Concurrency semantics for mutable global/module state must be specified before safe shared-thread execution is stabilized.

The compiler must not silently permit unsafe unsynchronized mutation merely because the program uses multiple threads.

---

## 29. Unsafe Concurrency

Low-level concurrent primitives may require:

```vut
unsafe:
  ...
```

when they bypass safe synchronization guarantees.

Unsafe concurrency behavior belongs jointly to:

```text
specs/19-ffi-unsafe.md
specs/20-concurrency.md
```

---

## 30. Deadlocks

Safe memory guarantees do not imply deadlock freedom.

If mutex-like synchronization is introduced, deadlocks may remain a logical program error unless stronger mechanisms are specified.

The standard library should document locking behavior clearly.

---

## 31. Determinism

Concurrency naturally introduces nondeterministic scheduling.

Vut should avoid unnecessary nondeterminism in APIs.

Operations whose result depends on execution ordering should document that behavior explicitly.

---

## 32. Testing Concurrency

Future concurrency implementation requires tests for:

* task completion
* cancellation
* synchronization
* race safety
* shutdown
* resource cleanup
* high contention
* platform differences

Concurrency tests should avoid relying solely on timing sleeps where deterministic synchronization can be used.

---

## 33. Sanitizers and Tooling

Compiler/runtime development should use available platform/toolchain analysis tools where useful to detect:

* races
* invalid memory behavior
* synchronization bugs

Exact tooling depends on the backend and target platform.

---

## 34. Concurrency and FFI

Foreign libraries may create threads or invoke callbacks from foreign threads.

Such behavior requires explicit FFI contracts.

A callback entering Vut from an arbitrary foreign thread must not be assumed safe without runtime support.

---

## 35. Deferred Syntax

The following syntax is intentionally not yet defined:

```text
spawn
thread
channel
select
atomic
mutex
rwlock
```

`async fn` and `await` are now defined by `specs/async/` for the single-thread
phase.

Codex/compiler implementation must not choose syntax for the still-deferred
concurrency features independently.

A future accepted language design must update:

```text
01-language-syntax.md
02-type-system.md
04-functions-methods.md
08-memory-model.md
11-runtime.md
20-concurrency.md
21-grammar.md
```

before implementing the deferred concurrency features.

---

## 36. Architectural Preparation

Even though concurrency is deferred, current architecture should:

* avoid global unsynchronized runtime state
* avoid assuming all execution occurs on one OS thread where unnecessary
* make runtime objects capable of future thread-safety review
* separate platform I/O implementations
* avoid compiler APIs that require process-global mutable state

This is preparation, not implementation of concurrency.

---

## 37. No MVP Blocker

Concurrency must not block implementation of:

```text
lexer
parser
AST
type system
data
functions
interfaces
modules
diagnostics
native codegen
VPM
stdlib core
```

These are higher priority for the initial Vut compiler.

---

## 38. Future Design Order

The single-thread async foundation establishes:

```text
1. async function syntax        (specs/async/01-syntax.md)
2. await syntax                 (specs/async/01-syntax.md)
3. async state-machine lowering (specs/async/03-lowering.md)
4. single-thread executor       (specs/async/04-runtime.md)
```

Before full concurrency becomes stable, the remaining design should be
finalized approximately in this order:

```text
1. thread-safety model
2. native thread API
3. synchronization primitives
4. task/spawn model
5. structured concurrency
6. cancellation API
7. multi-thread scheduler/runtime
8. async I/O reactors
```

This avoids adding multi-threaded concurrency before its underlying semantics
are understood.

---

## 39. Concurrency Principles

Vut concurrency follows these principles:

1. Concurrency is deferred from the initial MVP.
2. Core language execution is sequential by default.
3. Ordinary programs should not require an async runtime.
4. Concurrency must preserve memory safety.
5. Source-level value semantics remain valid across threads/tasks.
6. Parallel loops are not implicit.
7. Shared mutable state requires a defined synchronization model.
8. `async`/`await` is specified only as a single-thread foundation.
9. Typed communication is preferred over default `dyn`.
10. Task failures should integrate with Result-style errors.
11. Cancellation and resource cleanup must be explicitly defined.
12. Runtime implementation may reuse proven Rust concurrency libraries.
13. Rust implementation details must not dictate Vut syntax.
14. FFI concurrency requires explicit safety contracts.
15. Concurrency implementation must not delay the core Vut MVP.

This document intentionally defines architectural boundaries rather than a finalized concurrency syntax.
