# Vut FFI — Native ABI v1

This document is the normative specification of the **Public Native ABI v1** and
the **Vut-internal Runtime Handle ABI**. It is the source of truth for what may
cross an `extern "C"` boundary and how.

## 1. Layer separation

```text
Vut language representation
≠ Vut MIR representation
≠ Cranelift representation
≠ Public Native ABI v1
≠ Vut-internal Runtime Handle ABI
```

The Public Native ABI v1 is a stable C ABI. The Runtime Handle ABI is internal
to the compiler, stdlib, and official runtime and is **not** a public contract.

## 2. Public Native ABI v1

The only ABI exposed to arbitrary third-party C and native packages is C.

### 2.1 Scalar types

| Vut type | Native ABI |
|---|---|
| `i8` | signed 8-bit |
| `i16` | signed 16-bit |
| `i32` | signed 32-bit |
| `i64` | signed 64-bit |
| `u8` | unsigned 8-bit |
| `u16` | unsigned 16-bit |
| `u32` | unsigned 32-bit |
| `u64` | unsigned 64-bit |
| `usize` | target pointer width, unsigned |
| `isize` | target pointer width, signed |
| `f32` | IEEE binary32 |
| `f64` | IEEE binary64 |
| `bool` | the target C ABI's C `_Bool` (representation and calling convention defined by the target C ABI, not hard-coded to 1 byte) |
| `void` | return only |

`int` and `float` are **rejected** across FFI: their width and semantics are not
explicit enough for a stable ABI. Use fixed-width types.

If Vut ever needs a locked 1-byte boolean ABI, it must be defined as a Vut
`uint8` ABI with C-facing APIs using `uint8_t`, and must **not** be called
`_Bool`. Until then, `bool` follows C `_Bool` per target because the boundary is
`extern "C"`.

### 2.2 Pointers

`ptr[T]` has the target pointer size and alignment. Its pointee must be `void`,
a scalar, another pointer, a C function pointer, or an opaque/`@repr(C)` data
type. Pointers to managed handles are not a valid C type.

### 2.3 `@repr(C)` and opaque data

`@repr(C)` and opaque `data` cross the boundary **by pointer only** (`ptr[T]`).
By-value `data` is not part of ABI v1. `@repr(C)` preserves declaration field
order, uses C alignment and padding, and carries no hidden managed metadata.

### 2.4 Function pointers

`extern "C" fn(...) -> ...` callback types are allowed when every parameter and
the result are themselves boundary-safe. A callback value must be a
**non-capturing** function or lambda. Passing a capturing closure as a C callback
is a type error (a capturing closure is a tagged heap pointer, not a function
address).

### 2.5 Not exposed in v1

`map[K,V]`, `result[T,E]`, `T?`, `array[T,N]`, `enum`, by-value `data`,
`dyn`/`interface`, `future[T]`, `vutcon[T]`, and `channel[T]` are not part of the
public ABI. Expose them through opaque handles plus accessor functions.

## 3. Vut-internal Runtime Handle ABI

Used only between the compiler, the stdlib, and the official runtime.

- `str`, `bytes`, and `list[T]` cross as a single runtime-owned handle pointer.
  Parameters are **borrowed** for the call duration; returned handles
  **transfer ownership** to Vut. Vut retains/releases them with the runtime.
- `resource[T]` is a move-only owned opaque handle. It crosses the same way and
  keeps a single owner, released deterministically by Vut.
- Third-party C code must not use these representations. The documented
  alternative for a public boundary is `ptr[u8] + usize` length (see
  `specs/std/03-native-runtime.md`).

## 4. Ownership across FFI

| Case | Rule |
|---|---|
| Borrowed input | Managed handles and `ptr[T]`; valid for the call duration only. |
| Owned input | Deferred (no syntax in v1); native must not free Vut handles. |
| Owned return | Handles and `resource[T]` transfer to Vut; dropped deterministically. |
| Borrowed return | Deferred. |
| Resource handle | Move-only, single owner, released exactly once by Vut. |
| Callback capture | Must be non-capturing. |

Native code must not free Vut-managed memory with a different allocator. Within
one statically-linked image the process allocator is shared; a native library
must release Vut handles only through the runtime release functions.

## 5. Errors and unwinding

Native must not unwind across the C ABI boundary. Native failures are converted
to a Vut-visible form:

- `result[T,E]` / `T?` when the API can express them;
- a resource error state for fallible creation (see
  `specs/std/03-native-runtime.md`);
- a runtime trap for programmer/ABI violations.

There is no global mutable error slot.

## 6. Versioning

- Every exported runtime symbol is named `vut_rt_<module>_<operation>_vN`.
- The runtime exports `vut_rt_abi_version_v1() -> u32`; generated code references
  it so a compiler/runtime mismatch fails at link with a clear diagnostic.
- The installed `manifest.json:abi_version` is consumed by the build and by
  `vut doctor`.

## 7. Diagnostics

| Code | Meaning |
|---|---|
| `E8001` | extern call outside `unsafe` |
| `E8004` | non-FFI-safe type across the boundary |
| `E8005` | unknown ABI |
| `E8006` | invalid extern declaration or attribute |
| `E8007` | extern function with a Vut body |
| `E8013` (reserved) | capturing closure used as an FFI callback |
| `E8014` (reserved) | invalid `@repr(C)` field (managed handle, resource, etc.) |
