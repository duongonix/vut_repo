
# Vut FFI and Unsafe

## 1. Purpose

This document defines the direction and safety boundary for:

* native foreign-function interfaces
* C ABI integration
* raw pointers
* unsafe operations

FFI and unsafe are advanced features.

They are not required for the earliest Vut MVP compiler stages.

---

## 2. Safe by Default

Normal Vut code is safe.

Users should not need:

```text
raw pointers
manual allocation
manual free
pointer arithmetic
```

for ordinary applications.

Low-level operations require an explicit unsafe boundary.

---

## 3. Unsafe Boundary

The intended unsafe block syntax is:

```vut
unsafe:
  ...
```

Unsafe code is visually explicit through indentation.

The exact set of operations requiring `unsafe` is defined in this specification as features are finalized.

---

## 4. Unsafe Is Not a Type-System Escape Hatch

`unsafe` must not disable ordinary static typing.

Inside:

```vut
unsafe:
```

the compiler still checks:

* variable types
* function arguments
* syntax
* modules
* visibility
* control flow

Unsafe only permits explicitly designated operations that cannot be statically proven memory-safe.

---

## 5. Raw Pointer Type

The intended raw pointer type syntax is:

```text
ptr[T]
```

Examples:

```text
ptr[u8]
ptr[i32]
ptr[void]
```

Raw pointers are distinct from normal safe Vut values.

---

## 6. Null Raw Pointer

Raw pointer nullability semantics must be explicitly defined.

Do not automatically equate:

```text
ptr[T]
```

with:

```text
T?
```

Safe optional values and raw native pointers are distinct concepts.

---

## 7. Pointer Operations

Operations such as:

```text
dereference
pointer arithmetic
raw memory read
raw memory write
pointer conversion
```

must require unsafe context unless proven safe through a future dedicated abstraction.

Exact source syntax for these operations is not yet finalized.

Do not invent it before grammar specification is updated.

---

## 8. C FFI

Vut intends to support C ABI functions.

Conceptual syntax:

```vut
extern "C":
  fn malloc(size: u64) -> ptr[void]
```

Foreign declarations have no Vut body.

---

## 9. External Function Declaration

Conceptually:

```vut
extern "C":
  fn native_add(a: i32, b: i32) -> i32
```

Calling such a function may require `unsafe` depending on whether the compiler/stdlib can prove its safety contract.

For v1, arbitrary raw FFI calls should be considered unsafe by default.

---

## 10. Foreign Function Body

Invalid:

```vut
extern "C":
  fn native_add(a: i32, b: i32) -> i32:
    a + b
```

An external declaration identifies a foreign symbol rather than defining Vut implementation.

---

## 11. Supported ABI

Initial FFI should prioritize:

```text
"C"
```

Additional ABIs require explicit compiler/backend support and specification.

Do not accept arbitrary ABI strings silently.

---

## 12. FFI-Compatible Types

Only explicitly defined FFI-safe types may cross a raw C ABI boundary directly.

Candidates include fixed-width numeric types and `bool` (target C `_Bool`):

```text
i8 i16 i32 i64
u8 u16 u32 u64
f32 f64
bool
ptr[T]
```

High-level Vut types such as:

```text
map[K, V]
result[T, E]
T?
data
dyn
interface
```

must not automatically be assumed C ABI-compatible.

`bytes`, `str`, and `list[T]` cross only through the **Vut-internal Runtime
Handle ABI** (compiler/stdlib/official runtime). For arbitrary third-party C,
raw boundaries must use `ptr[u8]` plus an explicit length. See
`specs/ffi/06-native-abi-v1.md`.

---

## 13. General `int`

`int` should not automatically be used in a stable C ABI unless its exact ABI representation has been defined.

For FFI, prefer fixed-width types.

Example:

```vut
extern "C":
  fn calculate(value: i64) -> i64
```

---

## 14. Strings Across FFI

Vut `str` is not automatically equivalent to:

```text
char*
```

Users or standard-library bindings must explicitly convert to/from native string representations.

This conversion must define:

* encoding
* length
* ownership
* lifetime
* termination convention

---

## 15. Data Layout

Normal `data` layout is compiler-defined unless explicitly marked for a stable foreign layout by a future specification.

Do not assume:

```vut
data Point:
  x: i32
  y: i32
```

has C `struct` layout automatically.

A future representation annotation may be introduced only after being specified.

---

## 16. Memory Ownership Across FFI

Foreign memory ownership must be explicit in binding contracts.

Questions include:

```text
Who allocates?
Who frees?
Which allocator?
How long is memory valid?
Can the foreign function retain the pointer?
```

The compiler cannot infer these semantics from a raw pointer alone.

---

## 17. Foreign Allocators

If memory is allocated by a foreign allocator, it should normally be released using the matching foreign deallocator.

Example concept:

```text
C malloc -> C free
library_create -> library_destroy
```

Do not assume Vut runtime allocation/free functions are interchangeable with foreign allocators.

---

## 18. Callback FFI

Foreign callbacks require explicit ABI-compatible function representation.

Callback syntax and lifetime rules are not yet finalized.

Do not implement arbitrary callback conversion before this specification is expanded.

---

## 19. Dynamic Libraries

FFI may eventually support linking against:

```text
static libraries
dynamic libraries
system libraries
```

Library declaration/link configuration belongs primarily to package/build metadata rather than source syntax where possible.

---

## 20. Link Configuration

External libraries should preferably be declared through project/package configuration rather than hard-coded platform linker flags throughout source code.

Exact manifest fields are deferred to:

```text
specs/vpm/manifest.md
```

---

## 21. Unsafe Standard Library Internals

The standard library may implement safe abstractions using unsafe FFI.

Example concept:

```text
unsafe OS call
    ↓
validation
    ↓
safe std.fs API
```

Users of the safe wrapper should not need unsafe blocks.

---

## 22. Unsafe Interface

Unsafe behavior must not accidentally cross a supposedly safe API boundary.

A safe function must uphold its documented safety contract for all safe inputs.

If callers must maintain invariants manually, the function itself should be marked/treated unsafe once function-level unsafe declarations are finalized.

---

## 23. Function-Level Unsafe

The exact syntax for declaring an unsafe function is not yet locked.

Possible behavior requires specification before implementation.

Do not invent:

```text
unsafe fn
fn unsafe
```

until grammar and calling semantics are finalized.

For now, `unsafe:` blocks define the accepted surface direction.

---

## 24. Undefined Behavior

Unsafe code may make operations possible that cannot be protected by normal Vut guarantees.

The compiler should document which operations can lead to undefined behavior.

Safe Vut must not permit undefined behavior through ordinary well-typed code.

---

## 25. Unsafe Diagnostics

Unsafe-required operations used outside unsafe context must produce precise diagnostics.

Conceptual:

```text
error[E8001]: raw pointer operation requires `unsafe`
  --> src/native.vut:12:11
```

Help may suggest:

```vut
unsafe:
  ...
```

only when wrapping the operation is semantically appropriate.

---

## 26. Compiler Backend

FFI code generation should reuse the selected native backend's ABI/calling-convention support.

Do not implement C ABI lowering manually when the chosen backend already provides proven support.

---

## 27. Platform Differences

Foreign symbol names, libraries and ABIs may vary by platform.

FFI tooling must not assume Windows, Linux and macOS linkage are identical.

Platform-specific configuration should remain explicit where necessary.

---

## 28. Security

FFI is a trust boundary.

Raw foreign calls can violate:

* memory safety
* thread safety
* type invariants
* resource ownership

Unsafe syntax exists to make this boundary visible.

---

## 29. FFI/Unsafe Principles

1. Normal Vut is safe by default.
2. Low-level memory operations require explicit unsafe context.
3. `unsafe` does not disable static typing.
4. Raw pointer type uses `ptr[T]` direction.
5. C ABI is the initial FFI priority.
6. Fixed-width numeric types are preferred across FFI.
7. `str`, `data`, lists and interfaces are not automatically C-compatible.
8. Foreign ownership contracts are explicit.
9. Safe wrappers may hide unsafe internals.
10. Safe code must not expose undefined behavior.
11. Backend ABI support should be reused.
12. Unspecified FFI syntax must not be invented before the grammar is finalized.
