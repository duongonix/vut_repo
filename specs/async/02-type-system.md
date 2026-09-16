# Async Type System

## 1. Purpose

This document defines the static type behavior of `async fn` and `await`.

It must not contradict:

```text
specs/02-type-system.md
specs/04-functions-methods.md
specs/result/*.md
```

---

## 2. Logical Signature

For:

```vut
async fn foo() -> int:
  ...
```

the declared type is the **logical result type**:

```text
foo: async fn() -> int
```

The user never writes a future type.

---

## 3. Call Result Type

Calling an async function does not produce an `int` directly.

```vut
value = foo()
```

creates an internal awaitable value. Conceptually:

```text
foo() : internal future(int)
```

where `future(T)` is a compiler-internal type, not writable in Vut source.

Calling is distinct from computing:

```text
calling async fn
    !=
executing the function body to completion immediately
```

In this single-thread phase the compiler tracks the awaitable origin of an
async call internally. Awaitable values are not first-class: they cannot be
named, stored, or passed. An `await` must be applied directly to an async call:

```vut
value = await foo()
```

This restriction is lifted only when a real suspension source makes stored
awaitable values meaningful.

---

## 4. Internal Awaitable Type

Conceptually the compiler has an internal type constructor:

```text
future(T)
```

Properties:

```text
internal only
not nameable in source type syntax
not a standard-library type
no public constructor
no public operations other than `await`
```

In this phase the compiler may track awaitable origin structurally (the call
target is `async`) rather than materializing a distinct `future(T)` type.
Either representation is internal and must not appear in source syntax.

---

## 5. Await Typing

Rule:

```text
async_fn_call : future(T)
----------------------------
await async_fn_call : T
```

`await` requires an operand of internal awaitable type and yields the logical
result type.

Invalid:

```text
await 123        # int is not awaitable
await "text"     # str is not awaitable
```

Invalid:

```vut
async fn main():
  value = await 123
```

must be a compile error.

---

## 6. Context of `await`

`await` is valid only in one of these contexts:

```text
the body of an async function
the body of an async method
```

It is invalid:

```text
inside a normal fn
inside a normal method
inside a lambda
inside an interface requirement
at module top level
```

Invalid:

```vut
fn main():
  value = await load()
```

must be a compile error.

The check is lexical and local. An async function that contains a non-async
lambda may not use `await` inside that lambda.

---

## 7. Nested `await`

Nested await expressions type-check compositionally:

```vut
async fn combine() -> int:
  (await first()) + (await second())
```

Each `await` independently requires an awaitable operand and yields a logical
result.

---

## 8. Async Function Return Checking

The body of an async function is checked against its logical return type exactly
as a synchronous function is checked against its return type.

```vut
async fn load() -> int:
  data = await read()
  data
```

is valid only if every return path yields `int`.

An async function that returns an awaitable value without awaiting fails the
normal return-type check and is diagnosed as an async return problem:

```vut
async fn bad() -> int:
  read()        # read() : future(int), not int
```

with a help to add `await`.

---

## 9. Calling Async Functions

Because awaitable values are not first-class in this phase, an async function is
normally called only as the operand of `await`:

```text
await async_call(args)   valid
async_call(args)         awaitable value used as an ordinary value (E6014)
```

A normal (non-async) function cannot `await`, so it cannot directly consume an
async call; the compiler reports `E6011` for the `await` and `E6014` for the
unawaited call. There is no implicit "run and discard" behavior: an unawaited
async call is rejected rather than silently executed.

---

## 10. Async Functions Are Not Ordinary Function Values

There is no source-level asynchronous function type in this phase.

Therefore:

```text
async fn cannot be passed where fn(...) -> ... is expected
async fn cannot be stored in a variable of function type
async fn cannot be used as a callback
```

This keeps the function-type system unchanged. A future phase may define an
async function type explicitly.

---

## 11. Methods

Async methods use the same rules:

```vut
async fn Reader.read_async() -> bytes:
  ...
```

The receiver is an ordinary implicit `self`. Only the call result becomes an
internal awaitable value.

Async methods do not satisfy synchronous interface requirements, and interfaces
do not declare async requirements in this phase.

---

## 12. Result Interaction

`await` and `?` are independent.

```text
await
= wait for an async computation and take its logical result

?
= unwrap a success result or propagate its error
```

Given:

```vut
async fn read_async() -> result(bytes, FsError):
  ...
```

then:

```vut
data = await read_async()
```

has type:

```text
result(bytes, FsError)
```

and:

```vut
async fn load() -> result(bytes, FsError):
  data = await read_async()?
  ok(data)
```

type-checks as follows:

```text
read_async()          : future(result(bytes, FsError))
await read_async()    : result(bytes, FsError)
await read_async()?   : bytes
```

`?` uses the existing rule: the enclosing function must return a compatible
`result(_, E)`.

The two mechanisms must not be merged. `await` never propagates errors, and `?`
never waits for a computation.

---

## 13. Precedence in Typing

Because `await operation()?` parses as `(await operation())?`:

```text
operation()          : future(result(T, E))
await operation()    : result(T, E)
await operation()?   : T
```

The parser must retain this structure rather than folding `await` and `?`
together.

---

## 14. No Implicit Await

The compiler must never automatically await a value.

```text
no implicit await on assignment
no implicit await on return
no implicit await on function arguments
no implicit await in conditions
```

The programmer writes `await` explicitly.

---

## 15. No Implicit Conversion

The compiler must not silently convert a non-awaitable value into an awaitable
one, and must not coerce an awaitable value into its logical result without
`await`.

---

## 16. Recursion

Async functions may call themselves and each other. Each call creates a separate
awaitable value.

```vut
async fn countdown(n: int) -> int:
  if n <= 0:
    return 0

  await countdown(n - 1)
```

Each `await` drives the newly created awaitable value.

---

## 17. Type-System Principles

1. `async fn` exposes a logical result type.
2. The call result is an internal awaitable type.
3. `future(T)` is not source-nameable.
4. `await` requires an awaitable operand and yields the logical result.
5. `await` is valid only in async bodies.
6. `await` and `?` are independent.
7. No implicit await and no implicit conversion.
8. Async functions are not ordinary function values in this phase.
