# Async Diagnostics

## 1. Purpose

This document defines the diagnostics for `async fn` and `await`.

All diagnostics use the shared infrastructure:

```text
specs/09-errors-diagnostics.md
specs/22-error-codes.md
```

The compiler must not panic on invalid async source.

---

## 2. Diagnostic Codes

New codes assigned by this specification:

```text
E0112   Invalid `async` Modifier
E6011   `await` Outside `async fn`
E6012   Cannot Await Non-Awaitable Value
E6013   Invalid Async Return
E6014   Awaitable Value Used As Ordinary Value
E9006   Async Lowering Failure
W6002   Awaitable Value Dropped Without Await (reserved)
```

`E0111` is already assigned to attribute validation and must not be reused for
async. These codes are registered in `specs/22-error-codes.md`.

`W6002` is reserved for a future phase in which awaitable values are
first-class. In the current phase an unawaited async call is rejected as
`E6014`, so `W6002` is not emitted.

---

## 3. E0112 — Invalid `async` Modifier

Used when `async` is not followed by `fn`, or is used in an invalid position.

Example:

```vut
async data Point:
  x: int
```

```text
error[E0112]: `async` must be followed by `fn`
  --> src/main.vut:1:1
   |
 1 | async data Point:
   | ^^^^^ `async` must modify `fn`
```

---

## 4. E6011 — `await` Outside `async fn`

Example:

```vut
fn main():
  value = await load()
```

```text
error[E6011]: `await` is only valid inside an `async fn`
  --> src/main.vut:2:11
   |
 2 |   value = await load()
   |           ^^^^^^^^^^^^ `await` requires an enclosing `async fn`
   |
   = help: mark `main` as `async fn` or remove `await`
```

The primary span covers the await expression. A related location may point to the
enclosing non-async function.

---

## 5. E6012 — Cannot Await Non-Awaitable Value

Example:

```vut
async fn main():
  value = await 123
```

```text
error[E6012]: cannot await a value of type `int`
  --> src/main.vut:2:19
   |
 2 |   value = await 123
   |                   ^^^ `int` is not awaitable
   |
   = expected: awaitable async computation
   = found:    int
```

The compiler must not silently treat non-awaitable values as awaitable.

---

## 6. E6013 — Invalid Async Return

Used when an async function returns a value that is not compatible with its
logical return type, especially when an awaitable value was returned without
`await`.

Example:

```vut
async fn load() -> int:
  read()
```

where `read: async fn() -> int`.

```text
error[E6013]: invalid async return type
  --> src/main.vut:2:3
   |
 2 |   read()
   |   ^^^^^^ expected `int`, found an awaitable value
   |
   = expected: int
   = found:    awaitable result of an async call
   = help: add `await`: `await read()`
```

Ordinary return mismatches inside an async function may use the general
`E6005`; `E6013` is used when the mismatch is specifically an awaitable value.

---

## 7. E6014 — Awaitable Value Used As Ordinary Value

Used when an awaitable value is used where its logical result is required
without `await`.

Example:

```vut
async fn load() -> int:
  value = read()
  value + 1
```

```text
error[E6014]: awaitable value must be awaited before use
  --> src/main.vut:2:11
   |
 2 |   value = read()
   |           ^^^^^^ this is an awaitable value, not `int`
   |
   = help: write `value = await read()`
```

---

## 8. E9006 — Async Lowering Failure

Used for internal async lowering/verification failures.

```text
internal compiler error[E9006]: async lowering failed
```

This must not be reported as if the user wrote invalid source. It is a compiler
bug.

---

## 9. W6002 — Awaitable Value Dropped Without Await (Reserved)

Reserved for a future phase in which awaitable values are first-class.

In the current phase an unawaited async call is an error:

```vut
async fn main():
  load()
```

```text
error[E6014]: awaitable value must be awaited before use
  --> src/main.vut:2:3
   |
 2 |   load()
   |   ^^^^^^ this is an async computation, not an ordinary value
   |
   = help: write `await load()` to run the computation
```

`W6002` is therefore not emitted yet.

---

## 10. Ownership

Diagnostics are owned by the stage with the best knowledge:

```text
parser          E0112
type checker    E6012, E6013, E6014
async checker   E6011
async lowering  E9006
warnings        W6002 (reserved)
```

The `await`-outside-async check should be performed where the enclosing function
context is known, and reported once.

---

## 11. Rules

1. Invalid async source produces diagnostics, not panics.
2. Expected/found information is shown where meaningful.
3. Help is shown only when reliable.
4. Internal async failures use an internal code.
5. No cascading diagnostics from a single async root error.
