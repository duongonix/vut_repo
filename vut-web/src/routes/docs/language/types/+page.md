---
title: Types
description: 'Static typing with inference where it helps and explicit types where they matter.'
section: Language
---

## Static by default

Vut checks types during compilation. A normal variable has a fixed type, inferred from its first assignment or written as an annotation.

```vut
count: int = 10
name: str = "Vut"
active: bool = true
```

## Common types

| Type      | Purpose                  |
| --------- | ------------------------ |
| `int`     | Integer values           |
| `str`     | Text                     |
| `bool`    | `true` or `false`        |
| `dyn`     | Explicit dynamic values  |
| `list[T]` | A typed list of elements |
| `T?`      | An optional value        |

## Type compatibility

Reassignment must preserve a variable’s type. Function arguments and return values are also checked against their declared types.

```vut
fn square(value: int) -> int:
  value * value
```

## Optional values

An optional type can represent a value or `null`. A normal non-optional value does not silently accept `null`.

```vut
nickname: str? = null
```

## Explicit dynamic typing

Use `dyn` deliberately. The compiler never infers it as a workaround for incompatible assignments.

See [Variables](/docs/language/variables/#dynamic-values) for an example.
