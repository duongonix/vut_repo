---
title: FFI
description: 'Keep native interoperability explicit at a C-compatible boundary.'
section: Advanced
order: 4
---

## C ABI only

FFI v1 supports the C ABI. Rust, C++, and other languages must expose a C-compatible wrapper; their native ABIs are not accepted.

```vut
extern "C" fn add(a: i32, b: i32) -> i32
```

This is a declaration only. Linking and calling it requires the corresponding native library and the safety requirements of the FFI specification.

## Symbol names and layout

```vut
@link_name("native_add")
extern "C" fn add(a: i32, b: i32) -> i32

@repr(C)
data Point:
  x: f32
  y: f32
```

`@link_name` changes the native symbol name, not its ABI. Ordinary `data` declarations must not be assumed to have C layout.

## Ownership and safety

A native pointer does not tell the compiler how to release its allocation. Wrappers must pair creation with the native destroy/free operation and keep pointer lifetimes valid. Exceptions and panics must not unwind across the boundary.

## Managed values

Do not pass `str`, `bytes`, `list[T]`, `map[K, V]`, `dyn`, `result[T, E]`, optional values, interface values, or managed data directly through the FFI v1 boundary. Use explicitly ABI-safe types and conversions.

## Packaging

Keep raw declarations in a private module and expose a safe Vut API. See [Native libraries](/docs/advanced/native-libraries/) for artifact boundaries.
