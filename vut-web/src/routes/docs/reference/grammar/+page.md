---
title: Grammar
description: "A reading guide to Vut's core structural forms."
section: Reference
order: 17
---

## Blocks and declarations

A block starts with `:` and is delimited by indentation. Statements end at newlines. This page is a syntax guide, not a replacement for the repository's formal grammar specification.

```vut
data Point:
  x: int
  y: int

fn sum(a: int, b: int) -> int:
  a + b
```

## Expressions

Calls use parentheses, while parameterized types use brackets. Member access uses `.`, list literals use `@[...]`, and expression interpolation uses `$(...)` inside a string.

## Control flow

```text
if condition: block
elif condition: block
else: block

for: block
for condition: block
for value in iterable: block
for value, index in iterable: block
```

Here `block` is descriptive notation, not a literal keyword. See [Control flow](/docs/language/control-flow/) and [Loops](/docs/language/loops/) for examples.

## Excluded forms

Vut does not use braces for statement blocks, tuple literals, anonymous record literals, angle-bracket generic syntax, or `while`.
