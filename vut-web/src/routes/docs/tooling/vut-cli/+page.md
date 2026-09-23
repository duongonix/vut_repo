---
title: Vut CLI
description: 'Compile and run projects or standalone source files.'
section: Tooling
order: 5
---

## Usage

```text
vut run [source.vut] [-- program-arguments]
vut build [source.vut]
vut doctor [--target target]
```

The compiler exposes `run`, `build`, and `doctor`. Doctor checks toolchain prerequisites for a target without compiling an application. Package management, formatting, linting, and testing belong to VPM.

## Arguments

| Argument     | Meaning                                                               |
| ------------ | --------------------------------------------------------------------- |
| `source.vut` | Optional standalone entry source; project mode uses `src/main.vut`.   |
| `--`         | Separates arguments for the compiled program from compiler arguments. |

## Examples

```bash
vut run hello.vut
vut run app.vut -- hello world
vut build hello.vut
vut build --release
```

`run` compiles and executes. `build` produces native output without executing it. Release mode enables optimizations; development builds favor diagnostics and iteration.

## Options

```bash
vut --version
vut --help
vut run --help
vut build --help
```

The current CLI accepts `--release`, `-O/--opt-level 0..3`, `--target`, `-o/--output`, repeatable `--native-lib PATH`, and `--system-lib NAME` on build/run. Release implies O2 unless an explicit optimization level is supplied. Default artifacts are under `build/debug` or `build/release`.

Global options include `--color auto|always|never` and `--diagnostic-format human|json`. Use JSON diagnostics for tooling, not text scraping. Verify help when using an older installed toolchain.

## Exit status

Successful compilation returns zero. Compilation failures return non-zero; program failures propagate a non-zero status. Diagnostics preserve source locations and compiler error codes.
