---
title: Formatter
description: 'Consistent source formatting that preserves meaning.'
section: Tooling
order: 10
---

## Usage

```bash
vpm fmt
```

Formatting is a VPM workflow, not a `vut fmt` compiler subcommand. Consult the installed toolchain help for supported options and check-only mode.

## Canonical style

Use two-space indentation, newline-terminated statements, spaces around operators and assignments, and a space after commas. Blocks remain colon-and-indentation based.

```vut
fn add(a: int, b: int) -> int:
  a + b
```

## Guarantees

Formatting must preserve program semantics and comments, and be idempotent: formatting an already formatted file should produce no further changes. Formatting is not permission to rewrite control flow or alter types.
