# Async Testing

## 1. Purpose

This document defines the required tests for the async/await phase.

Per repository rules, this phase needs enough verification to confirm the
foundation works. Exhaustive async testing belongs to a later hardening phase.

---

## 2. Phase-Level Verification

Minimum confirmation for this phase:

```text
async fn parses
await parses
basic await type checking works
await outside async fn is rejected
multiple awaits type-check
async main is supported
Result + await type-checks
compiler does not crash
workspace build/tests pass
```

---

## 3. Parser Tests

```text
async fn free function
async fn method
await prefix expression
return await call()
process(await call())
nested await
await operation()? precedence
rejection of `op().await`
rejection of async blocks
rejection of `async` on non-function declarations
```

---

## 4. Type Tests

Compile-pass:

```text
async fn returning a primitive
async fn returning a data value
await on an async call
await on a parenthesized async call
await inside nested expressions
await operation()? with result types
async method called and awaited
async main
```

Compile-fail:

```text
await in a normal fn
await on int / str / literal
await on a future in a lambda
storing an async call in a local (E6014)
returning an awaitable without await
using an awaitable as an ordinary value
async fn used where a function value is expected
```

---

## 5. Lowering and MIR Tests

```text
async function lowers to a state machine
one state per await site
locals live across await are stored
completion state is reachable
cleanup is inserted on return, `?`, and completion
internal verification passes
```

---

## 6. Codegen and Runtime Tests

```text
async fn with no await runs to completion
async fn with one await runs to completion
async fn with multiple awaits runs in order
async main executes and returns the expected exit status
single-thread executor drives the root future
no threads are spawned
```

---

## 7. Ownership Tests

```text
managed local live across an await is dropped once
managed local dropped on early return
managed local dropped on `?` error path
unawaited future dropped without running the body
future dropped while suspended releases resources
nested async calls release resources
live-allocation count returns to baseline
```

Run these with the existing ownership suite and, where available, sanitizers.

---

## 8. Regression Tests

Async parser and semantic errors get compile-fail regression tests with stable
codes:

```text
E0111
E6011
E6012
E6013
E6014
```

Internal async lowering failures get an internal regression test when a
reduction is found.

---

## 9. Workspace Verification

After implementation, run:

```text
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

plus the project's native build/run integration tests.

---

## 10. Testing Rules

1. Confirm the foundation before hardening.
2. Every async error code has a compile-fail test.
3. Async ownership has leak/double-drop coverage.
4. `async main` has an end-to-end test.
5. Full async stress testing is deferred to the hardening phase.
