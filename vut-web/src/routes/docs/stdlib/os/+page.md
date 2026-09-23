---
title: Operating system
description: 'Platform integration without pretending every OS is identical.'
section: Standard library
order: 22
---

## Responsibility

The standard-library architecture assigns OS integration to `os`, while file access belongs to `fs`, lexical path manipulation to `path`, and environment access to `env`.

## Cross-platform behavior

Platform-specific capabilities must be exposed honestly. A cross-platform surface must not silently equate Windows and Unix semantics.

## Platform queries

```vut
import os

fn main():
  out(os.name(), os.arch(), os.family())
  out(os.cpu_count())
  match os.current_dir():
    ok(directory): out(directory)
    err(error): out(error.message)
```

`name()`, `arch()`, and `family()` return strings. `cpu_count()` returns `usize`. Do not equate a CPU count with a guarantee that every task runs simultaneously.

## Paths and working directory

`home_dir()`, `temp_dir()`, `current_dir()`, and `current_exe()` return `result[str, OsError]`. `set_current_dir(path: str)` returns `result[unit, OsError]`.

Changing the working directory affects relative-path resolution for the process; prefer explicit paths when independent components run concurrently. Failure to query a directory is an error, not a fabricated empty path.
