# Phase 21 - Async/Await Single-Thread Foundation

## Status

Complete (single-thread foundation)

`async fn` / `await` are specified under `specs/async/` and implemented for the
single-thread, no-suspension phase.

## Goal

Add `async fn` and `await` with a single-threaded execution model, without
introducing threads or parallel execution.

## Scope

```text
async fn
await
async state-machine lowering
single-thread executor
async main
ownership across suspension
```

Explicitly out of scope:

```text
threads
worker pools
channels
mutex / rwlock / atomics
task.spawn
structured concurrency
parallel execution
async networking / filesystem
select / join / race / timeouts
```

## Tasks

- [x] Specify syntax, typing, lowering, runtime, memory, and diagnostics.
- [x] Reserve `async` and `await` in the lexer.
- [x] Parse `async fn`, `async fn Type.method`, and prefix `await`.
- [x] Represent async declarations and awaits in AST and HIR.
- [x] Type-check async signatures, awaitable tracking, and `await`.
- [x] Lower `async fn` and `await` for the single-thread phase.
- [x] Accept `async main` without a runtime driver.
- [x] Preserve ownership across the (non-suspending) await.
- [x] Emit the async diagnostics.

## Tests

- [x] `async fn` and `await` parse.
- [x] Basic await type checking.
- [x] `await` outside `async fn` is rejected.
- [x] Multiple awaits type-check.
- [x] `async main` runs to completion.
- [x] Result + await (`await operation()?`) type-checks and runs.
- [x] `examples/async.vut` compiles and runs.
- [x] Compiler does not crash on invalid async source.
- [x] Workspace build and tests pass.

## Benchmarks

None yet.

## Decisions

Async is single-threaded. `await` is a prefix expression valid only inside
`async fn`. `await` binds tighter than postfix `?`. Async functions expose a
logical result type; the awaitable value is internal and not first-class, so an
`await` operand must be a direct async call. Because no await can suspend, the
compiler lowers awaited calls inline; the state-machine/executor model remains
the normative design for future suspension.

## Known Limitations

No threads, parallel execution, channels, or async I/O. Full async stress and
sanitizer testing belongs to a later hardening phase.
