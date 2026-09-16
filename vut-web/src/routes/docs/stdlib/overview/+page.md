---
title: Standard library
description: 'Understand the boundary between language primitives, official libraries, and packages.'
section: Standard library
order: 8
---

## Three layers

| Layer            | Responsibility                                                                                            |
| ---------------- | --------------------------------------------------------------------------------------------------------- |
| Core             | Types such as `str`, `bytes`, `list(T)`, `map(K, V)`, and `result(T, E)`, plus memory/runtime primitives. |
| Standard library | Official system and general-purpose modules distributed with Vut.                                         |
| VPM packages     | Independently evolving functionality such as HTTP, JSON, databases, and frameworks.                       |

## Initial modules

The standard-library design prioritizes I/O, paths, filesystems, OS integration, environment access, time, processes, math, randomness, and FFI. Module availability depends on the installed toolchain; an entry in the architecture is not a release claim.

## Implementation boundary

High-level behavior belongs in Vut when practical. Native support crosses a stable C ABI to a shared runtime, rather than requiring a separate native library for every module.

## Reference status

Detailed release-verified API examples for [fs](/docs/stdlib/fs/), [path](/docs/stdlib/path/), [os](/docs/stdlib/os/), and [time](/docs/stdlib/time/) are tracked separately. Their pages clearly distinguish specified responsibilities from verified installation support.
