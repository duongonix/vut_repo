---
title: Keywords
description: 'A compact reference for the documented Vut keyword surface.'
section: Reference
order: 9
---

## Core syntax

| Purpose                 | Keywords                                  |
| ----------------------- | ----------------------------------------- |
| Declarations            | `fn`, `data`, `interface`, `enum`, `type` |
| Branching               | `if`, `elif`, `else`, `match`             |
| Loops                   | `for`, `in`, `break`, `continue`          |
| Return                  | `return`                                  |
| Imports                 | `import`, `as`, `at`                      |
| Boolean logic           | `and`, `or`, `not`                        |
| Literal values          | `true`, `false`, `null`                   |
| Explicit dynamic typing | `dyn`                                     |

## Specialized syntax

`async` and `await` provide suspension. `vut` schedules a Vutcon; `vutcon[T]` is a built-in type, not another spawn keyword. `static` declares a type-level method. FFI uses `extern "C" fn`, `opaque data`, and `unsafe` blocks. `composition` and `children` are not current lexer keywords; they belong to the superseded Vutcom design.

These mechanisms have their own constraints; see the Advanced section before using them.

## Deliberate exclusions

There is no `while`: use `for condition:`. There are no `let`, `var`, or `const` declarations. ALL-CAPS bindings are constants. Privacy follows leading underscores, not `pub` or `private`. Equality uses `==`, not `is`.

## Types and operators

See [Built-in types](/docs/reference/builtin-types/) and [Operators](/docs/reference/operators/) rather than treating every built-in name as a keyword.
