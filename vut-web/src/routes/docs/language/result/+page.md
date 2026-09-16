---
title: Result
description: 'Construct, match, and propagate expected failures.'
section: Language
order: 6
---

## Two states

`result(T, E)` is a core type, not a standard-library enum. Its active state is either `ok(value)` or `err(error)`.

```vut
fn checked(value: int) -> result(int, str):
  if value < 0:
    return err("expected a non-negative value")
  ok(value)

fn twice(value: int) -> result(int, str):
  number = checked(value)?
  ok(number * 2)

fn main():
  match twice(21):
    ok(answer): out("$answer")
    err(message): out(message)
```

## Propagation rules

For an operand of type `result(T, E)`, postfix `?` produces `T` on success. On failure, it returns an error from the current function. That function must return `result(_, E2)` with a compatible error type. There is no implicit error conversion.

## Async results

`await operation()?` means `(await operation())?`: awaiting and result propagation are independent mechanisms.

## Expected failures

Use results for application failures a caller can handle. Panics remain distinct and should not replace ordinary error handling.
