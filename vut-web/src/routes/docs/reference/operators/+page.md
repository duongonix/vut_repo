---
title: Operator reference
description: 'A compact table of documented operators and punctuation.'
section: Reference
order: 18
---

## Operators

| Form                             | Meaning                                      |
| -------------------------------- | -------------------------------------------- |
| `+`, `-`, `*`, `/`, `%`          | Arithmetic.                                  |
| `==`, `!=`, `<`, `<=`, `>`, `>=` | Comparison.                                  |
| `and`, `or`, `not`               | Boolean logic.                               |
| `=`                              | Assignment and named construction arguments. |
| `..`, `..=`                      | Exclusive and inclusive ranges.              |
| `.`                              | Member or module access.                     |
| `->`                             | Function return type.                        |
| `?` after a result expression    | Success extraction or error propagation.     |
| `?` after a type                 | Optional type.                               |
| `=>`                             | Expression-bodied anonymous callable.        |

## Context matters

`@(...)` constructs a list. `$(...)` interpolates an expression inside a template string. They are not interchangeable record constructors.

## Grouping and await

Use parentheses for explicit grouping. `await operation()?` parses as `(await operation())?`. See [Operators](/docs/language/operators/) for introductory examples.
