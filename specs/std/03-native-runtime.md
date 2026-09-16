# Vut Standard Library — Native Runtime

## 1. Purpose

`vut-runtime` provides the native primitives required by the official Vut standard library.

It is not the high-level standard library.

Architecture:

```text
Vut stdlib
↓
native bridge
↓
stable C ABI
↓
Rust vut-runtime
↓
Operating System
```

---

## 2. Runtime Library

All official native stdlib support is compiled into one static library.

Windows:

```text
vut-runtime.lib
```

Linux/macOS:

```text
libvut-runtime.a
```

Do not create one native library per stdlib module.

---

## 3. Symbol Naming

All exported runtime symbols use:

```text
vut_rt_<module>_<operation>
```

Examples:

```text
vut_rt_fs_exists
vut_rt_fs_read
vut_rt_fs_write

vut_rt_os_home_dir

vut_rt_time_now

vut_rt_process_spawn
```

Avoid arbitrary symbol naming.

---

## 4. Vut Native Declaration

Native functions are declared internally using stable C ABI.

Example:

```vut
@link_name("vut_rt_fs_exists")
extern "C" fn _native_exists(
  path: ptr(u8),
  path_len: usize
) -> bool
```

The exact signature must match the native ABI contract.

Native declarations should normally remain under `_internal`.

---

## 5. Rust Export

Rust exports corresponding C ABI functions.

Conceptually:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_fs_exists(
    path: *const u8,
    path_len: usize,
) -> bool {
    // ...
}
```

Do not expose Rust ABI directly.

Do not expose Rust traits, `Future`, `String`, `Vec<T>`, `Result<T, E>`, or other Rust-specific layouts across the ABI.

---

## 6. ABI-Safe Types

Prefer simple ABI-safe representations:

```text
fixed-width integers
usize/isize where target-compatible
floats
raw pointers
explicit C-compatible structures
status/error codes
```

Managed Vut types require explicit conversion.

---

## 7. Strings

Vut `str` must not be passed as an undocumented runtime object.

At the native boundary, use an explicit representation such as:

```text
pointer to UTF-8 bytes
+
length
```

Native functions must not retain borrowed Vut pointers after returning unless the ABI explicitly establishes ownership transfer.

UTF-8 validity and ownership rules must remain clear.

---

## 8. Bytes and Buffers

`bytes` is a managed Vut type.

Native APIs returning buffers must explicitly define:

```text
who allocates
who owns
length
capacity if relevant
who destroys the allocation
behavior on error
```

Never create an ABI where ownership is ambiguous.

For operations such as:

```text
fs.read
process output
network receive
```

reuse a common buffer ownership mechanism instead of inventing a different mechanism for every module.

---

## 9. Errors

Do not pass Rust error objects through C ABI.

Native functions should expose stable status/error information.

Conceptually:

```text
native operation
↓
status/error code
↓
Vut internal conversion
↓
domain error
↓
result(T, E)
```

Example:

```text
OS ERROR_FILE_NOT_FOUND
or errno ENOENT

↓ native mapping

stable native status

↓ Vut mapping

FsErrorKind.not_found
```

Platform-specific error codes must not unnecessarily leak into normal public APIs.

---

## 10. Memory Ownership

Native functions must follow the Vut memory model.

Never introduce:

```text
use-after-free
double-free
implicit leaks
ambiguous ownership
invalid retained pointer
```

For every native pointer crossing the boundary, the implementation must know whether it is:

```text
borrowed
owned by Vut
owned by runtime
transferred
```

Unsafe Rust should remain minimal and isolated.

---

## 11. Native Buffers

Prefer a small number of reusable ABI patterns.

Examples:

```text
borrowed input:
(ptr, len)

caller-owned output buffer:
(ptr, capacity) → written length

runtime-owned returned buffer:
explicit ownership transfer + matching release mechanism
```

Do not design a unique buffer ABI for every stdlib function.

---

## 12. Platform Implementation

Prefer Rust standard-library APIs where they provide correct semantics.

Examples:

```rust
std::fs
std::env
std::process
std::time
```

Use Win32/libc/platform-specific APIs only when required.

Platform-specific behavior should be isolated in focused modules.

---

## 13. Linking

`@link_name` identifies a symbol.

It does not identify the library file.

Example:

```vut
@link_name("vut_rt_fs_read")
extern "C" fn _native_read(...)
```

The compiler/build system automatically includes:

```text
~/.vut/lib/runtime/<target>/vut-runtime.lib
```

or:

```text
~/.vut/lib/runtime/<target>/libvut-runtime.a
```

in the native LinkPlan.

---

## 14. Runtime Installation

Canonical location:

```text
~/.vut/lib/runtime/<target>/
```

Example:

```text
~/.vut/lib/runtime/x86_64-pc-windows-msvc/vut-runtime.lib
```

This library supports the official stdlib as a whole.

---

## 15. Third-Party Packages

Third-party native libraries are different from the official runtime.

They live inside their package:

```text
~/.vut/packages/<package>/<version>/
├── vpm.toml
├── src/
└── native/
    └── <target>/
        └── *.lib / *.a
```

Do not put third-party package libraries inside:

```text
~/.vut/lib/runtime/
```

---

## 16. Development

During development, a locally built runtime may be selected through:

```text
VUT_RUNTIME_PATH
```

with installed runtime fallback:

```text
~/.vut/lib/runtime/<target>
```

This avoids requiring a GitHub Release for every development iteration.

---

## 17. Native Runtime Rule

Before adding a native function, ask:

> Does this operation genuinely require native/runtime support?

If it can cleanly and efficiently be implemented in Vut, implement it in Vut.

The native runtime must remain a focused platform/runtime primitive layer, not a second implementation of the standard library.
