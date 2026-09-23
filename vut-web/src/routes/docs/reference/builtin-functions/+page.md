---
title: Built-in functions
description: 'Core operations without inventing an unverified API catalog.'
section: Reference
order: 19
---

## Result constructors

`ok(value)` and `err(error)` construct the two states of `result[T, E]`. Matching uses the same names. See [Result](/docs/language/result/).

## Concurrent work

`vut(callable)` is the reserved spawning form for a parameterless anonymous callable in an async body. It produces `vutcon[T]`, even when its handle is discarded. See [Vutcon](/docs/advanced/vutcon/).

## Core methods

Documented explicit conversions include `str.to_bytes()` and `bytes.to_str()`. The latter returns `result[str, Utf8Error]` because decoding can fail. Methods are not global function aliases.

## Output

`out(...values)` prints values, separating multiple arguments with spaces and ending with a newline. It accepts supported scalar and structured values; it is not a C-style format-string function. Use interpolation to build text and [I/O](/docs/stdlib/io/) for fallible stream writes.

## Numeric methods

Integer and default floating-point values provide `abs()`, `min(other)`, `max(other)`, and `clamp(low, high)`. Integer `pow(exponent)` uses saturating arithmetic; integer `abs()` retains wrapping behavior at the minimum representable value.

Default float methods include `pow`, `floor`, `ceil`, `round`, `trunc`, `sqrt`, `is_nan`, `is_finite`, and `to_int`. Float-to-int conversion saturates, with NaN mapping to zero. Do not assume the same method surface exists for `f32`.

Use `to_str()` for scalar text conversion, `int.to_float()` for floating conversion, and `int.to_char()` for scalar-to-text conversion when appropriate. Boolean `to_str()` produces text; booleans do not provide truthiness for arbitrary values.

## Collection constructors

`bytes()` constructs a buffer; `bytes.from_list(...)` and `bytes.from_hex(...)` are typed constructors. `channel[T](capacity: n)` creates a channel. List, array, and map literals have their own syntax, not a shared generic constructor function.

See [Lists and arrays](/docs/language/lists-arrays/), [Maps](/docs/language/maps/), and [Strings and bytes](/docs/language/strings-bytes/) for the method catalog.
