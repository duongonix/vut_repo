# Vut Compiler CLI

## 1. Purpose

This document defines the command-line interface of the Vut compiler executable:

```text
vut
```

The `vut` CLI is intentionally small.

It has exactly two primary commands:

```text
vut run
vut build
```

Project and package management belong to `vpm`.

---

## 2. CLI Philosophy

`vut` is a compiler-facing tool.

It should not become a second package manager.

The following responsibilities belong to `vut`:

```text
compile source
run compiled source
build native output
```

Everything related to package/project workflow belongs to `vpm`.

---

## 3. Primary Commands

The two commands are:

```text
vut run
vut build
```

Do not introduce compiler commands such as:

```text
vut add
vut install
vut update
vut publish
vut fmt
vut lint
vut test
```

Those belong to `vpm`.

---

## 4. `vut run`

Run a Vut program:

```powershell
vut run
```

Inside a standard project, the default entry point is:

```text
src/main.vut
```

The command compiles the program and executes it.

---

## 5. Run a File

A standalone Vut source file may be run directly:

```powershell
vut run hello.vut
```

The compiler uses that file as the entry source.

---

## 6. Program Arguments

Arguments intended for the compiled program are separated using:

```text
--
```

Example:

```powershell
vut run -- hello world
```

File:

```powershell
vut run app.vut -- hello world
```

Everything after `--` belongs to the target Vut program rather than the compiler.

---

## 7. `vut build`

Build the current project:

```powershell
vut build
```

Default project entry:

```text
src/main.vut
```

The command produces native output without automatically executing it.

---

## 8. Build a File

A standalone source file may be built:

```powershell
vut build hello.vut
```

---

## 9. Debug Build

The default build mode should favor development.

Conceptually:

```text
debug
```

Debug builds prioritize:

- useful diagnostics
- compile speed
- debug information
- runtime checks where appropriate

---

## 10. Release Build

Release mode uses:

```powershell
vut build --release
```

Release builds prioritize optimized native output.

Possible differences include:

- optimization enabled
- reduced debug metadata
- stronger inlining
- dead-code elimination
- runtime devirtualization
- other backend optimizations

---

## 11. Run Release

Release execution may use:

```powershell
vut run --release
```

The compiler builds the release configuration and executes the result.

---

## 12. Target

Cross-compilation may use a flag such as:

```powershell
vut build --target <target>
```

Example concept:

```powershell
vut build --target x86_64-pc-windows-msvc
```

Supported target names should follow the compiler backend/toolchain conventions where practical.

---

## 13. Output

An explicit output path may be supported through:

```powershell
vut build --output <path>
```

Example:

```powershell
vut build main.vut --output build/app.exe
```

Exact platform extension handling should be automatic where possible.

---

## 14. Default Build Directory

Project builds should use a dedicated build directory.

Recommended structure:

```text
build/
├── debug/
└── release/
```

Example Windows output:

```text
build/debug/app.exe
build/release/app.exe
```

Example Unix output:

```text
build/debug/app
build/release/app
```

Internal compiler caches may live elsewhere.

---

## 15. Help

Global help:

```powershell
vut --help
```

Command help:

```powershell
vut run --help
vut build --help
```

Help text must remain concise and reflect actual implemented behavior.

---

## 16. Version

Compiler version:

```powershell
vut --version
```

This is a flag, not a command.

Do not require:

```text
vut version
```

---

## 17. Color

Diagnostic color control should support conventional modes.

Conceptually:

```powershell
vut build --color auto
vut build --color always
vut build --color never
```

Default:

```text
auto
```

---

## 18. Diagnostic Format

Human-readable diagnostics are the default.

Tooling should also be able to request structured diagnostics.

Conceptually:

```powershell
vut build --diagnostic-format json
```

Exact flag naming may be refined by implementation as long as the capability remains available.

---

## 19. Exit Codes

Successful compilation/execution:

```text
0
```

Compilation failure:

```text
non-zero
```

Runtime program failure should propagate an appropriate non-zero exit code.

Internal compiler failure must also return non-zero.

Exact exit-code categories may be documented separately.

---

## 20. Compiler Errors

Errors use the diagnostic format defined in:

```text
specs/09-errors-diagnostics.md
```

Example:

```text
error[E1003]: type mismatch
  --> src/main.vut:12:9
   |
12 |   age = "20"
   |         ^^^^ expected `int`, found `str`
```

---

## 21. No Package Network Operations

`vut` must not automatically:

- search registries
- download packages
- update packages
- publish packages

Those are VPM responsibilities.

The compiler consumes already-resolved source roots/dependencies.

---

## 22. Project Mode vs File Mode

Project mode:

```powershell
vut build
vut run
```

uses the current project's configuration/environment.

File mode:

```powershell
vut build hello.vut
vut run hello.vut
```

operates on the specified source entry.

The compiler should keep behavior predictable between these modes.

---

## 23. VPM Integration

VPM may invoke compiler APIs or the `vut` executable.

Example:

```text
vpm build
    ↓
resolve project/dependencies
    ↓
invoke Vut compiler
```

Likewise:

```text
vpm run
    ↓
resolve project/dependencies
    ↓
compile
    ↓
run
```

VPM remains responsible for project orchestration.

---

## 24. Thin CLI

The `vut` executable should be a thin layer around reusable compiler libraries.

Avoid implementing compiler logic directly inside command handlers.

Conceptually:

```text
CLI
 ↓
Compiler API
 ↓
Compiler pipeline
```

This allows reuse by:

- VPM
- tests
- language servers
- future IDE tooling

---

## 25. CLI Dependencies

The Rust implementation should use an established CLI parsing library rather than writing argument parsing manually.

The chosen dependency should support:

- subcommands
- flags
- help generation
- validation

Do not build a custom argument parser unless a concrete limitation requires it.

---

## 26. CLI Principles

The `vut` CLI follows these principles:

1. Only two primary commands: `run` and `build`.
2. `--version` and `--help` are flags.
3. Package management belongs to VPM.
4. `run` compiles and executes.
5. `build` compiles without execution.
6. Both project and standalone-file workflows are supported.
7. `--` separates program arguments.
8. Debug is the normal development mode.
9. Release builds enable optimization.
10. Diagnostics follow the common Vut diagnostic system.
11. Structured diagnostics are available for tooling.
12. CLI remains a thin wrapper around compiler APIs.

This document is normative for the `vut` command-line interface.