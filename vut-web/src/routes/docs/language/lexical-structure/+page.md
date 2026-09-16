---
title: Lexical structure
description: 'Files, indentation, comments, and template strings.'
section: Language
order: 1
---

## Files and blocks

Vut source files end in `.vut`. Every file is a module; no module declaration is required. Newlines terminate statements. A colon starts an indented block, and the canonical formatter uses two spaces per level.

## Comments

```vut
# A line comment
##
A block comment
spans several lines.
##
### Documentation for the next declaration
fn greet():
  out("Hello")
```

## Identifiers

Identifiers contain letters, digits, and underscores but cannot start with a digit. Names are case-sensitive. A leading underscore marks module privacy; ALL-CAPS bindings are constants.

## Strings and interpolation

Double-quoted strings are templates. `$name` inserts an identifier; `$(expression)` inserts a normal, type-checked expression. Escape a literal dollar sign with `\$`.

```vut
fn double(value: int) -> int:
  value * 2

fn main():
  name = "Vut"
  out("Hello, $name. Answer: $(double(21))")
  out("A literal dollar: \$")
```

`$()` is reserved for string interpolation, not anonymous records. Named structured values use `data`.
