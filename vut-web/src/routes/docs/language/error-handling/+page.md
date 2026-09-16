---
title: Error handling
description: 'Distinguish missing values, expected errors, and violated invariants.'
section: Language
order: 1
---

## Choose the right type

Use `T?` for a value that may be absent, and `result(T, E)` for an operation with an expected error. Neither changes ordinary values into `dyn`.

## Handle a result

A result holds exactly one active state: `ok(value)` or `err(error)`. Match both cases explicitly.

```vut
fn divide(a: int, b: int) -> result(int, str):
  if b == 0:
    return err("division by zero")
  ok(a / b)

fn main():
  match divide(10, 2):
    ok(value): out("Result: $value")
    err(error): out("Error: $error")
```

## Propagate failures

The postfix `?` operator unwraps a successful result or returns its error from the enclosing compatible result-returning function. See [Result](/docs/language/result/).

## Absence is not an error

```vut
nickname: str? = null
```

See [Optional values](/docs/language/optional/) for the documented nullable surface. A plain `str` cannot receive `null`.

## Programmer errors

Results model expected failures. They do not replace panics for violated invariants. Do not conceal a failure by substituting a default value.
