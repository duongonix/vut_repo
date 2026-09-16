---
title: Built-in functions
description: 'Core operations without inventing an unverified API catalog.'
section: Reference
order: 19
---

## Result constructors

`ok(value)` and `err(error)` construct the two states of `result(T, E)`. Matching uses the same names. See [Result](/docs/language/result/).

## Concurrent work

`vut(callable)` is the reserved spawning form for a parameterless anonymous callable in an async body. It produces `vutcon(T)`, even when its handle is discarded. See [Vutcon](/docs/advanced/vutcon/).

## Core methods

Documented explicit conversions include `str.to_bytes()` and `bytes.to_str()`. The latter returns `result(str, Utf8Error)` because decoding can fail. Methods are not global function aliases.

## Catalog status

A complete release-verified catalog of global built-ins is not yet published here. Core specification examples such as assertion and numeric helpers may describe design direction, not guaranteed installed APIs. Consult the selected toolchain's documentation before depending on them.
