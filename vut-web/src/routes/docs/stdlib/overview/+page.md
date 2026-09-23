---
title: Standard library
description: 'Understand the boundary between language primitives, official libraries, and packages.'
section: Standard library
order: 8
---

## Three layers

| Layer            | Responsibility                                                                                            |
| ---------------- | --------------------------------------------------------------------------------------------------------- |
| Core             | Types such as `str`, `bytes`, `list[T]`, `map[K, V]`, and `result[T, E]`, plus memory/runtime primitives. |
| Standard library | Official system and general-purpose modules distributed with Vut.                                         |
| VPM packages     | Independently evolving third-party libraries, databases, and frameworks.                                  |

## Initial modules

The checked-in standard library provides `io`, `path`, `fs`, `os`, `env`, `time`, `process`, `http`, and `json`. Public imports omit the source-directory name: write `import fs`, not `import std.fs`. Use selected imports such as `import json at Value` for type annotations.

## Implementation boundary

High-level behavior belongs in Vut when practical. Native support crosses a stable C ABI to a shared runtime, rather than requiring a separate native library for every module.

## Reference status

These pages describe the current repository's public Vut modules, checked against examples and compiler integration points. They do not claim that every older distributed toolchain includes the same APIs. Match your compiler and stdlib versions.

Start with [I/O](/docs/stdlib/io/), [files](/docs/stdlib/fs/), [paths](/docs/stdlib/path/), [OS](/docs/stdlib/os/), [environment](/docs/stdlib/env/), [time](/docs/stdlib/time/), or [processes](/docs/stdlib/process/). [HTTP](/docs/stdlib/http/) and [JSON](/docs/stdlib/json/) are official modules in this checkout, not hypothetical third-party packages.
