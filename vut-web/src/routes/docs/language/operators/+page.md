---
title: Operators
description: 'Arithmetic, comparison, logic, ranges, and propagation.'
section: Language
order: 3
---

## Arithmetic and comparison

Arithmetic uses `+`, `-`, `*`, `/`, and `%`. Equality uses `==` and `!=`; ordering uses `<`, `<=`, `>`, and `>=`. Operands must satisfy the type system.

```vut
fn is_even(value: int) -> bool:
  value % 2 == 0
```

## Boolean logic

Use `and`, `or`, and `not`, not `&&`, `||`, or `!`. Conditions require `bool`; numbers and strings are not implicitly truthy.

```vut
fn allowed(age: int, banned: bool) -> bool:
  age >= 18 and not banned
```

## Ranges

`0..10` excludes the upper bound. `0..=10` includes it. Both can be used in `for` loops.

## Result propagation

Postfix `?` unwraps or propagates a result inside a compatible result-returning function. It is distinct from the optional-type suffix in `T?`. See [Result](/docs/language/result/).

## Reference

See the [operator reference](/docs/reference/operators/) for a compact table. Parenthesize expressions when grouping would otherwise be unclear.
