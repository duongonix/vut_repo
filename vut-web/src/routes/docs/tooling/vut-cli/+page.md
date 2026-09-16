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
```

The compiler has two primary commands: `run` and `build`. Package management, formatting, linting, and testing belong to VPM.

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

Consult your installed toolchain's command help for supported target, output, color, and diagnostic-format flags. The CLI specification describes these capabilities, but not every proposed flag is a stable release guarantee.

## Exit status

Successful compilation returns zero. Compilation failures return non-zero; program failures propagate a non-zero status. Diagnostics preserve source locations and compiler error codes.
