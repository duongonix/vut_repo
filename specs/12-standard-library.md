# Vut Standard Library

## 1. Purpose

This document defines the philosophy, organization and minimum scope of the Vut standard library.

The standard library is referred to as:

```text
std
```

Detailed module contracts live under:

```text
specs/std/
```

---

## 2. Standard Library Goals

The Vut standard library should be:

- small
- practical
- predictable
- cross-platform
- strongly typed
- consistent with Vut syntax
- efficient
- modular

It should provide common functionality without attempting to contain every possible application library.

---

## 3. Core vs Ecosystem

Functionality required by most programs belongs in `std`.

Specialized functionality should normally live in VPM packages.

Examples that belong in the ecosystem rather than core `std` may include:

```text
web frameworks
database drivers
GUI frameworks
machine learning
game engines
specialized file formats
cloud SDKs
```

---

## 4. Standard Library Modules

Initial specification areas:

```text
std/
├── core
├── string
├── io
├── fs
├── env
├── math
├── time
└── result-option
```

These correspond to:

```text
specs/std/core.md
specs/std/string.md
specs/std/io.md
specs/std/fs.md
specs/std/env.md
specs/std/math.md
specs/std/time.md
specs/std/result-option.md
```

---

## 5. Core

Core functionality includes fundamental operations required by normal Vut programs.

Potential responsibilities include:

- basic types
- comparison support
- conversions
- iteration contracts
- fundamental utility functions
- Result/optional integration

Core APIs should remain especially stable.

---

## 6. Collections

Collection support includes:

```text
list[T]
map[K, V]
bytes
```

Collection APIs should follow consistent naming and behavior.

Do not introduce multiple unrelated APIs for equivalent operations.

The standard `list[T]` operations (`map`, `filter`, `fold`, `any`, `all`,
`find_index`, `sort_by`, `join`, ...) are **builtins of the compiler**, not
standard-library free functions, so no `import collections` is required. See
`specs/collections.md` for the two implementation tiers (native runtime
primitives vs. compiler-lowered higher-order builtins).

---

## 7. Strings

`str` functionality should include common operations such as:

- length
- search
- split
- replace
- trim
- case conversion
- prefix/suffix checks
- conversion
- iteration where specified

Strings are UTF-8.

String APIs must clearly distinguish where operations refer to:

- bytes
- Unicode scalar values
- user-visible characters

when that distinction matters.

---

## 8. Method-Oriented APIs

Where natural, standard-library operations should use methods.

Example style:

```vut
text.trim()
text.to_int()
items.first()
items.slice(1, 5)
```

Avoid unnecessary global functions when an operation naturally belongs to a value.

---

## 9. Indexing

Vut does not use square-bracket indexing.

Do not design:

```vut
items[0]
```

The intended API style is:

```vut
items.at(0)
```

Slicing should use methods such as:

```vut
items.slice(1, 5)
```

Exact safe/optional/error behavior is defined by the collection specification.

---

## 10. Type Conversion

Conversions should be explicit.

Intended style:

```vut
value.to_int()
value.to_float()
value.to_str()
```

Conversions that may fail should return a typed failure representation rather than silently producing invalid values.

---

## 11. I/O

Standard I/O APIs should cover:

```text
stdin
stdout
stderr
printing
reading
```

Basic output should be simple:

```vut
print("Hello")
```

The basic API is exactly `print()`, `out()`, and `input()`. `print(value)`
writes without a trailing newline; `out(value)` writes with one trailing
newline; `input(prompt: str) -> str` writes and flushes its prompt without a
newline, reads one stdin line, removes its line ending, and returns `str`. All
three support typed template strings. Do not add `println`, `printf`, `echo`,
or `console.log` as core APIs.

`print` and `out` accept **zero or more** arguments of any displayable type
(scalars, `str`, `bytes`, `bool`, `list`, `array`, `data`, enums, `result`, and
opaque handles). Arguments are formatted with the compiler-generated display
conversion (see `specs/display.md`) and joined by single spaces.

```vut
out(User(a: 1))          # data
out(@[1, 2, 3])           # list
out(1, true, "x")         # 1 true x
out()                     # a single newline
out("user=$user")         # interpolation uses the same conversion
```

Display is resolved entirely at compile time: there is no reflection, no
runtime type metadata, and no runtime name lookup.

More advanced APIs belong to `std.io`.

---

## 12. File System

File-system support should provide cross-platform APIs for:

- reading files
- writing files
- directories
- metadata
- paths
- existence checks
- file operations

Expected failures should use typed errors.

---

## 13. Environment

Environment APIs may include:

- environment variables
- command-line arguments
- current working directory
- executable information

Platform differences should be normalized where practical.

---

## 14. Math

Math APIs should include commonly required numeric operations.

Examples may include:

```text
abs
min
max
sqrt
pow
sin
cos
round
floor
ceil
```

Exact APIs are defined in:

```text
specs/std/math.md
```

---

## 15. Time

Time APIs should distinguish concepts such as:

- duration
- instant
- wall-clock time
- date/time

Avoid representing every time concept as an untyped integer.

---

## 16. Result

Expected recoverable failures should use typed Result-style values.

Conceptually:

```text
result[T, E]
```

A successful operation contains `T`.

A failed operation contains `E`.

Exact representation is defined in:

```text
specs/std/result-option.md
```

---

## 17. Optional Values

Optional values use:

```text
T?
```

Example:

```vut
name: str? = null
```

Optional APIs must prevent unsafe implicit access.

Exact unwrapping/narrowing behavior must be specified before implementation is finalized.

---

## 18. Error Propagation

Vut intends to support concise Result propagation using:

```vut
value = operation()?
```

Exact semantics must be defined in `result-option.md` and the grammar before implementation.

Do not substitute exceptions for this mechanism.

---

## 19. No Exception-Based Standard Library

Standard-library APIs should not rely on exception-driven normal control flow.

Operations such as:

```text
read file
parse integer
open socket
```

should expose expected failure through typed return values.

---

## 20. Iteration

Collections should share a consistent iterable model.

This enables:

```vut
for value in items:
  ...
```

and:

```vut
for value, index in items:
  ...
```

The iterable contract must integrate with structural interfaces or another statically defined protocol.

---

## 21. Naming

Standard-library names should be:

- concise
- lowercase where appropriate
- consistent
- unsurprising

Avoid unnecessarily verbose APIs.

Example preference:

```text
read
write
open
close
exists
remove
```

over deeply verbose naming.

---

## 22. Cross-Platform Behavior

Where possible, the same Vut API should behave consistently on:

- Windows
- Linux
- macOS

Platform-specific behavior should be exposed only where abstraction would be misleading.

---

## 23. Platform-Specific APIs

When a feature genuinely only exists on certain platforms, its availability must be explicit.

The compiler or library should provide useful diagnostics rather than failing unexpectedly at runtime where static detection is possible.

---

## 24. Standard Library and Runtime

`std` and the runtime are distinct.

The runtime contains low-level compiler support.

`std` contains user-facing APIs.

Example:

```text
runtime internal file primitive
        ↓
std.fs
        ↓
Vut program
```

Runtime symbols must not automatically become public standard-library APIs.

---

## 25. Standard Library Implementation

The standard library may combine:

- Vut source
- Rust runtime bindings
- platform-specific native support

High-level functionality should preferably be implemented in Vut where practical.

Low-level functionality may use runtime/native implementations.

---

## 26. Unsafe Internals

The standard library may use unsafe/native internals to implement safe APIs.

Unsafe implementation does not make the public API unsafe if the library correctly maintains its safety contract.

Unsafe code should remain isolated and audited.

---

## 27. No Hidden Dynamic Typing

Standard-library APIs should preserve static types.

Do not use `dyn` merely to simplify library implementation when a generic or typed API is appropriate.

Example:

```text
list[int]
```

should retain integer specialization rather than internally forcing every element into a dynamic object at the language level.

---

## 28. API Stability

Standard-library APIs are part of Vut's compatibility surface.

Breaking changes require versioning policy consideration.

Core APIs should be especially conservative once Vut reaches stable releases.

---

## 29. Documentation

Every public standard-library API should eventually provide:

- description
- parameters
- return type
- error behavior
- examples
- platform limitations where relevant

Documentation should be usable by:

```text
vpm doc
```

and future editor tooling.

---

## 30. Testing

Standard-library modules require:

- unit tests
- error-case tests
- cross-platform tests where applicable
- integration tests
- regression tests

Unsafe implementations require additional safety-focused tests.

---

## 31. Performance

Common standard-library operations should avoid unnecessary allocations and dynamic dispatch.

Performance-sensitive APIs should be benchmarked.

Optimization must not compromise safety or semantic consistency.

---

## 32. Standard Library Principles

The standard library follows these principles:

1. Keep `std` focused on broadly useful functionality.
2. Specialized libraries belong in VPM packages.
3. APIs remain strongly typed.
4. Expected failures use typed results.
5. Avoid exception-driven APIs.
6. Prefer method syntax where natural.
7. No `[]` indexing.
8. Explicit conversion is preferred.
9. Cross-platform behavior should be consistent.
10. Runtime internals remain separate from public APIs.
11. Unsafe internals may power safe public APIs.
12. APIs require documentation and tests.
13. Avoid unnecessary runtime allocation and dynamic dispatch.

This document defines the overall standard-library contract. Individual APIs are specified under `specs/std/`.
