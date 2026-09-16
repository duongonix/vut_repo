# Vut Standard Library — Architecture

## 1. Architecture

The standard library has two implementation layers:

```text
Vut layer
+
Native runtime layer
```

The normal flow is:

```text
Application
↓
Public Vut stdlib API
↓
Internal Vut implementation
↓
Native bridge when required
↓
vut-runtime
↓
OS
```

Not every stdlib function requires native code.

---

## 2. Vut Layer

High-level stdlib modules live under:

```text
std/
```

Example:

```text
std/fs/
├── mod.vut
├── file.vut
├── directory.vut
├── metadata.vut
├── options.vut
├── error.vut
└── _internal/
    ├── mod.vut
    ├── native.vut
    ├── convert.vut
    └── constants.vut
```

This exact structure is not mandatory for every module.

Small modules should remain small.

Do not create unnecessary files merely to follow a template.

---

## 3. Public API

`mod.vut` should primarily define or expose the public module API.

Example:

```vut
import fs

if fs.exists("config.txt"):
  ...
```

Public APIs should not expose native implementation details.

Users should never need to know whether an operation is implemented in Vut, Rust, Win32, libc, or another native mechanism.

---

## 4. Internal Modules

Implementation-specific code should be placed under:

```text
_internal/
```

Typical responsibilities:

```text
native.vut
    extern declarations

convert.vut
    native ↔ Vut conversions

constants.vut
    internal constants
```

Native bridge functions are internal implementation details and must not become normal public stdlib APIs.

---

## 5. Native Runtime

Native implementation lives under:

```text
native/vut-runtime/src/
```

Example:

```text
native/vut-runtime/src/
├── lib.rs
├── fs/
│   ├── mod.rs
│   ├── api.rs
│   ├── error.rs
│   ├── buffer.rs
│   ├── windows.rs
│   └── unix.rs
├── os/
│   ├── mod.rs
│   ├── api.rs
│   ├── windows.rs
│   └── unix.rs
└── ...
```

Split native modules by responsibility.

Do not place the entire runtime into `lib.rs`.

---

## 6. Rust Responsibilities

Rust should implement only functionality requiring native/runtime access.

Appropriate examples:

```text
filesystem primitives
environment access
process primitives
OS information
system clock primitives
native networking primitives
thread primitives
terminal primitives
```

Rust should not unnecessarily implement:

```text
high-level validation
convenience wrappers
normal collection transformations
high-level Result handling
ordinary path manipulation that can safely be implemented in Vut
```

---

## 7. Platform Abstraction

Prefer Rust's standard library when it provides suitable cross-platform semantics.

Examples:

```rust
std::fs
std::env
std::path
std::process
std::time
```

Use platform-specific implementations only when necessary.

Platform-specific code should be isolated.

Example:

```text
fs/
├── api.rs
├── windows.rs
└── unix.rs
```

Do not scatter:

```rust
#[cfg(target_os = "...")]
```

through unrelated code when a clean platform module can isolate the difference.

---

## 8. Dependency Direction

Preferred dependency direction:

```text
public API
↓
internal Vut implementation
↓
native declarations
↓
native runtime
```

Do not make native runtime code depend on Vut high-level implementation details.

Do not create circular dependencies between stdlib modules.

Foundational modules should remain reusable by higher-level modules.

---

## 9. Module Responsibilities

Each module must have a clear responsibility.

Examples:

```text
path
    path manipulation and representation

fs
    filesystem operations

os
    operating-system information

env
    environment variables

process
    child process management

time
    time and duration APIs
```

Do not turn `os` or `io` into generic dumping grounds.

---

## 10. Compiler Relationship

The compiler should understand only what is necessary to resolve and compile stdlib modules.

High-level APIs such as:

```vut
fs.exists(...)
fs.read(...)
process.run(...)
```

must not be compiler intrinsics merely for convenience.

The stdlib should behave as normal Vut code wherever possible.

---

## 11. Development Overrides

During local stdlib development, the compiler may resolve:

```text
VUT_STDLIB_PATH
→ fallback ~/.vut/std
```

and:

```text
VUT_RUNTIME_PATH
→ fallback ~/.vut/lib/runtime/<target>
```

This allows local development without publishing a release after every change.

---

## 12. Architecture Rule

When deciding where functionality belongs:

```text
Language semantics?
→ Core/compiler

General high-level functionality shipped with Vut?
→ std/

Requires native/OS primitive?
→ native/vut-runtime/

Independent ecosystem library?
→ VPM package
```

Do not cross these boundaries merely to reduce implementation effort.
