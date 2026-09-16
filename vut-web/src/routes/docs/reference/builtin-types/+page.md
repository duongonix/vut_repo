---
title: Built-in types
description: 'Primitive types and the canonical parameterized type notation.'
section: Reference
order: 10
---

## Primitive types

| Types                     | Meaning                                                                |
| ------------------------- | ---------------------------------------------------------------------- |
| `bool`                    | `true` or `false`.                                                     |
| `i8`, `i16`, `i32`, `i64` | Signed integers.                                                       |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers.                                                     |
| `int`                     | Default general-purpose integer.                                       |
| `f32`, `f64`, `float`     | Floating-point values; `float` is the default.                         |
| `str`                     | Unicode text.                                                          |
| `bytes`                   | Managed binary buffer with `u8` elements; not an alias for `list(u8)`. |
| `void`                    | No returned value.                                                     |
| `dyn`                     | Explicit dynamic value; never silently inferred.                       |

## Type application

Parameterized types use parentheses, not angle brackets.

```vut
numbers: list(int) = @(1, 2, 3)
nickname: str? = null
```

Other documented forms include `map(K, V)`, `result(T, E)`, `ptr(T)`, `vutcon(T)`, and `vutcom(D)`. Each has its own construction and ownership rules.

## Explicit conversions

A `str` is not implicitly interchangeable with `bytes`. Encoding uses `text.to_bytes()`; decoding uses `data.to_str()` and returns a result because UTF-8 may be invalid.

## Numeric constraints

An annotated literal must fit its target type: `value: u8 = 300` is invalid. Do not assume an ABI representation for `int` or `float` from their names alone.
