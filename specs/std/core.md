
# Vut Standard Library — Core

## 1. Purpose

`core` defines the smallest fundamental APIs required by ordinary Vut programs.

It should remain:

* small
* stable
* fast
* dependency-light
* platform-independent where possible

`core` must not become a dumping ground for unrelated utilities.

---

## 2. Core Language Types

The language/compiler owns primitive types:

```text
bool

i8 i16 i32 i64
u8 u16 u32 u64

int
float
f32
f64

str
bytes

dyn
null
```

Standard library provides behavior around these types.

Core also provides the builtin `Utf8Error` data type returned by
`bytes.to_str()`:

```text
data Utf8Error:
  valid_up_to: int
  error_len: int
```

---

## 3. Conversion

Preferred API style:

```vut
value.to_int()
value.to_float()
value.to_str()
```

Conversions must be explicit when information may be lost or parsing may fail.

---

## 4. Numeric Conversion

Examples:

```vut
a = 10
b = a.to_float()
```

Narrowing conversions must not silently truncate when Vut semantics require validation.

Fallible conversion should return typed failure.

---

## 5. String Conversion

Common values should support:

```vut
value.to_str()
```

where a meaningful canonical representation exists.

Do not require `dyn` for formatting ordinary statically typed values.

---

## 6. Boolean

Boolean values are:

```vut
true
false
```

No truthiness.

Conversions to `bool` must not silently interpret arbitrary integers, strings, lists, or objects as truthy/falsy.

---

## 7. Null

`null` is primarily associated with optional values:

```vut
value: User? = null
```

Do not expose null as a general substitute for typed values.

---

## 8. Assertions

Core should provide an assertion facility.

Conceptually:

```vut
assert(condition)
```

and optionally:

```vut
assert(condition, "message")
```

Failed assertion represents programmer failure, not ordinary recoverable control flow.

---

## 9. Panic

A low-level panic facility may exist:

```vut
panic("message")
```

Use panic for unrecoverable programming/runtime failure.

Expected failures should use:

```text
result[T, E]
```

### Runtime bounds panic (implementation contract)

The compiler and runtime share one panic path for invalid indexes:

```text
vut_rt_panic_v1(message_ptr: *const u8, message_len: usize) -> !
vut_rt_bounds_panic_v1(op_id: i64, index: i64, length: i64) -> !
vut_rt_bounds_check_v1(op_id: i64, index: i64, length: i64) -> i64
```

- `op_id` selects a human-readable operation name from a single table (for
  example `list.at`, `bytes.set`, `array.at`); per-type panic entry points are
  not introduced.
- `vut_rt_bounds_panic_v1` formats `<op>: index <index> out of range (length <length>)`
  and delegates to `vut_rt_panic_v1`.
- `vut_rt_bounds_check_v1` is used by generated code: it traps for an invalid
  index and otherwise returns the index unchanged.
- `vut_rt_panic_v1` writes the message to stderr and aborts the process. It does
  not unwind across the ABI and does not run destructors: an invalid index is a
  programming error, so terminating is correct and cannot double-free.

---

## 10. Min / Max

Numeric helpers may include:

```vut
min(a, b)
max(a, b)
```

They must remain statically typed.

---

## 11. Clamp

Conceptually:

```vut
value.clamp(min, max)
```

or a consistent equivalent.

Exact placement should follow API consistency.

---

## 12. Type Introspection

General reflection is not part of Vut v1.

Do not expose broad runtime reflection through `core`.

Limited internal type metadata required by `dyn` is not automatically public API.

---

## 13. Memory

Safe Vut code must not expose:

```text
malloc
free
raw pointer arithmetic
```

through normal `core`.

Low-level memory belongs to unsafe/FFI facilities.

---

## 14. Iteration

`core` may define compiler-known iteration contracts used by:

```vut
for value in items:
```

User-facing iterator APIs must remain minimal until iterator protocol semantics are finalized.

---

## 15. Comparison

Core comparison behavior must align with language operators:

```text
==
!=
<
>
<=
>=
```

No `is`.

---

## 16. Hashing

Hashing required by maps should use an efficient implementation.

Do not expose unstable internal hash algorithms as language guarantees unless explicitly specified.

---

## 17. Compiler Intrinsics

Some core operations may lower to compiler intrinsics.

Intrinsics must be:

* centralized
* documented internally
* type checked
* inaccessible through arbitrary forged names

Do not scatter compiler magic across std modules.

---

## 18. Zero-Cost Operations

Primitive arithmetic/comparison should lower directly to efficient native operations whenever possible.

Do not route primitive operations through dynamic runtime dispatch.

---

## 19. Implementation

`core` may combine:

```text
compiler intrinsics
Vut source
small Rust runtime functions
```

Use the lowest abstraction that preserves safety and performance.

---

## 20. Rules

1. `core` stays small.
2. Primitive operations remain statically typed.
3. No truthiness.
4. No general reflection.
5. No safe manual memory management.
6. Expected errors use typed results.
7. Panic is for unrecoverable failures.
8. Avoid hidden allocations.
9. Avoid hidden `dyn`.
10. Core APIs must remain highly stable.




