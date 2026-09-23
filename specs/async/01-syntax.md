# Async Syntax

## 1. Purpose

This document defines the canonical surface syntax of `async fn` and `await`.

No alternative syntax is finalized.

---

## 2. Reserved Keywords

This phase reserves exactly two new keywords:

```text
async
await
```

`async` is a declaration modifier. `await` is a prefix expression operator.

Both must be reserved by the lexer before any other async behavior is
implemented.

---

## 3. `async fn`

An asynchronous free function is declared by prefixing `fn` with `async`:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

The declared return type is the **logical result type**.

An asynchronous method uses the same unified function model:

```vut
async fn Counter.refresh() -> bool:
  value = await fetch()
  value
```

`async` applies only to function and method declarations. It is not valid on
`data`, `interface`, `enum`, `type`, or as a standalone statement.

---

## 4. Logical Signature

The user-facing signature stays logical:

```vut
async fn fetch() -> Data:
```

The user is not required to write:

```vut
fn fetch() -> future[Data]:
```

Internally the compiler may lower the declaration to a future/state-machine
representation.

Calling an `async fn` does not execute the body to completion immediately. It
produces an awaitable value. The body is driven by `await`.

---

## 5. `await` Prefix Expression

`await` is a prefix expression:

```vut
value = await operation()
```

Allowed contexts include:

```vut
value = await operation()

return await operation()

process(await operation())

a = await first()
b = await second(a)
b
```

`await` evaluates its operand, suspends until the async computation completes,
and produces the logical result.

---

## 6. Nested `await`

`await` may appear inside any expression position that accepts an expression:

```vut
async fn combine() -> int:
  (await first()) + (await second())
```

There is no separate syntax for nested awaits.

---

## 7. Forbidden Syntax

The following are not part of Vut async syntax:

```text
operation().await
await(...)         # unless it is awaiting a parenthesized expression
async { ... }
async do { ... }
async fn expression
```

Canonical form is always:

```vut
await operation()
```

Postfix `.await` is never valid.

`async` is a declaration modifier, not a block or expression form.

---

## 8. Parenthesized Operand

Because parentheses group expressions, this is simply `await` applied to a
grouped expression:

```vut
result = await (choose())
```

It is not a distinct call-like `await(...)` syntax. Formatters and style should
prefer:

```vut
result = await choose()
```

---

## 9. Precedence

`await` binds tighter than the postfix result-propagation operator `?`, and
tighter than all binary operators.

Therefore:

```vut
value = await operation()?
```

parses as:

```text
(await operation())?
```

meaning:

```text
await the async computation
then propagate a Result error with `?`
```

`await` and `?` remain independent mechanisms.

---

## 10. Precedence Relative to Unary Operators

`await` is a unary-like prefix operator and is the operand of ordinary unary
operators:

```vut
not await is_ready()
-await score()
```

The operand of `await` is a postfix expression (member access and calls), not a
bare unary expression. `await -x` is not meaningful and is a parse error.

---

## 11. `await` Placement Rules

`await` is only valid inside the body of an `async fn`.

Invalid:

```vut
fn main():
  value = await load()
```

must be a compile error.

`await` inside a plain `fn()` lambda is invalid, even when the lambda appears
inside an async function. A lambda is async only when explicitly written as
`async fn()`:

```vut
make = fn():
  await load()          # invalid: plain `fn()` lambda is sync

make = async fn():
  await load()          # valid: anonymous async fn
```

The arrow form `() => expr` is always sync. `async () => ...` does not exist.

---

## 12. Operand Rules

The operand of `await` must be an awaitable value recognized by the compiler.

In this phase an awaitable value is produced only by calling an `async fn`, and
it is not first-class. Therefore the canonical operand is a direct async call,
optionally parenthesized:

```vut
await operation()
await (operation())
```

Awaitable values cannot be stored in locals in this phase. Ordinary values are
not awaitable.

Invalid:

```vut
async fn main():
  value = await 123
```

must be a compile error.

The compiler must never silently convert a non-awaitable value into an
awaitable one.

---

## 13. `async main`

The conventional executable entry point may be asynchronous:

```vut
async fn main():
  data = await load()
  out("$data")
```

The compiler/runtime drives `async main` to completion using the single-thread
executor. The user does not write:

```text
block_on(...)
run_executor(...)
```

Entry-point rules otherwise follow `specs/04-functions-methods.md`: `main` has
no parameters and a logical return type of `void` or `int`.

---

## 14. Grammar Summary

```text
function_declaration :=
    [ "async" ] "fn" identifier
    "(" [ parameter_list ] ")"
    [ "->" type_expression ]
    block

method_declaration :=
    [ "async" ] "fn" qualified_type_name "." identifier
    "(" [ parameter_list ] ")"
    [ "->" type_expression ]
    block

postfix_expression :=
    primary_expression
    { member_access | call_suffix | result_propagation }

primary_expression :=
    ...
  | await_expression

await_expression :=
    "await" await_operand

await_operand :=
    primary_expression { member_access | call_suffix }

result_propagation := "?"
```

`await_expression` is a primary expression, so `?` and later member access apply
after the await. The base of an await operand is not itself an `await`; nested
awaits use parentheses or call arguments.

The formal grammar document (`specs/21-grammar.md`) is normative.

---

## 15. Formatting

The canonical formatter prints:

```vut
async fn name(...) -> T:
  body
```

and:

```vut
await expression
```

with a single space after `await`.

---

## 16. Syntax Principles

1. `async` is a modifier on `fn`.
2. `await` is a prefix expression.
3. `.await` is not Vut syntax.
4. `async` blocks are not part of this phase. Anonymous `async fn` lambdas are
   supported (see `specs/async/08-vutcon.md`); the arrow form stays sync.
5. `await` is only valid in an `async fn`, including an anonymous `async fn`.
6. `await` binds tighter than `?`.
7. `async main` is allowed and driven by the runtime.
