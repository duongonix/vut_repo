# Vut Collections & Method Surface

Source of truth for the built-in methods of `str`, `bytes`, `list(T)`,
`array(T,N)`, `map(K,V)`, numerics, `bool`, and `result(T,E)`. Phase A adds the
methods marked **[A]**; methods marked later are planned and not yet available.

## Builtin implementation tiers

A **builtin** is a compiler-known API. Builtin does *not* imply a native runtime
function. Collection builtins come in two tiers:

1. **Native runtime primitives** — operations that manipulate a collection's
   memory/layout directly. They dispatch through the versioned C ABI
   (`vut_rt_list_*_v1`, ...). The runtime reuses the element layout and the
   compiler-generated retain/release callbacks; no callback is ever invented for
   them. Examples: `push`, `pop`, `at`, `set`, `extend`, `reverse`, `sort`,
   `truncate`, `swap`, `shrink_to_fit`, `index_of`, `join`.
2. **Compiler-lowered higher-order builtins** — operations that accept a
   callback. The type checker knows their signatures, and MIR lowers them into
   ordinary loops that invoke a **normal Vut function value** (`CallIndirect`).
   No runtime callback ABI exists for them; semantics live before the backend.
   Examples: `map`, `filter`, `fold`, `any`, `all`, `find_index`, `sort_by`.

Higher-order builtins take non-capturing function values (named functions or
lambdas that only use their own parameters). They are builtins, not standard
library functions: no `import collections` is ever required.

## Ordering contract (`list.sort()` / `array.sort()`)

- Supported element types: `int`, `i8..i64`, `u8..u64`, `usize/isize`, `float`,
  `f32/f64`, `str`, `bool`.
- Ascending order. Complexity is O(n log n) and in place.
- Stability is **unspecified** for `list.sort()` / `array.sort()`.
- `bool`: `false < true`.
- `str`: lexicographic by Unicode scalar value.
- `float`: ascending numeric order; `-0.0` and `+0.0` compare equal; all NaNs
  sort after all non-NaN values; NaNs compare equal to each other for sorting.
- Any other element type is rejected at compile time; use `list.sort_by`.

## Bounds (indexing) contract

An explicit index request (`at`, `set`, `insert`, `remove`) is a claim that the
index is valid. An invalid index is a **programming error** and must **trap**
through the shared runtime bounds-panic path (`vut_rt_bounds_panic_v1`, see
`specs/std/core.md` §9). The compiler and runtime must never fabricate a
zero/default element for an invalid index.

| Operation | Invalid index behavior |
| --- | --- |
| `list.at(i)`, `array.at(i)` | trap |
| `list.set(i, v)`, `array.set(i, v)` | trap |
| `list.insert(i, v)` when `i > len` | trap |
| `list.remove(i)` | trap |
| `bytes.at(i)`, `bytes.set(i, v)` | trap |

`array` uses a compile-time length, so a literal out-of-range index is rejected
at compile time (`E1003`); a runtime index is checked before the access.

Accessors with their own contracts keep them and do **not** follow strict
indexing:

- `list.first()`, `list.last()`, `list.pop()` on an empty list return the
  established zeroed fallback.
- `str.char_at(i)` returns `""` when out of range.
- `list.slice` / `bytes.slice` keep their documented clamping behavior.

A trap abort does not run destructors: it terminates the process immediately
(no unwinding across the ABI), so no double-free or partial release can occur.

## `list(T)`

`len`, `is_empty`, `capacity`, `reserve`, `push`, `at`, `set`, `insert`,
`remove`, `clear`, `slice`, `contains`,
**[A]** `pop`, `first`, `last`, `index_of`, `extend`, `reverse`, `sort`,
`truncate`, `swap`, `shrink_to_fit`,
`join` (`list(str)` only),
`map`, `filter`, `fold`, `any`, `all`, `find_index`, `sort_by`.

### Native runtime methods

- `pop() -> T` removes and returns the last element; on an empty list it is the
  established bounds behavior (zeroed element).
- `index_of(value: T) -> int` returns the first matching index or `-1`.
- `extend(other: list(T))` appends every element of `other`.
- `join(separator: str) -> str` — only available on `list(str)`. Joining any
  other element type is a compile error (`E1028`); map the elements to `str`
  first. An empty list joins to `""`; there is no trailing separator.

### Compiler-lowered higher-order methods

Callbacks are non-capturing function values. Each method lowers to a loop; the
callback is invoked through the normal Vut call mechanism (no runtime ABI).

```text
map(fn(T) -> R) -> list(R)          filter(fn(T) -> bool) -> list(T)
fold(U, fn(U, T) -> U) -> U         any(fn(T) -> bool) -> bool
all(fn(T) -> bool) -> bool          find_index(fn(T) -> bool) -> int
sort_by(fn(T, T) -> int) -> void
```

- `map` preallocates the result to the input length.
- `filter` preallocates to the input length (upper bound).
- `fold` threads the accumulator with no intermediate collection.
- `any` / `all` / `find_index` **short-circuit** on the first decisive element.
- `find_index(predicate)` returns the first matching index, or `-1`; it is
  distinct from `index_of(value)`, which searches by value (both return `-1`
  when absent).
- `sort_by(compare)` sorts **in place**, is **stable**, and is O(n log n)
  (iterative bottom-up merge sort). `compare` returns `<0`, `0`, or `>0`;
  `<= 0` keeps the left element, which is what makes the sort stable.
- `map` / `filter` / `fold` / `any` / `all` / `find_index` / `sort_by` work on
  any element type, including managed types (`str`, `bytes`, nested `data`).

## `map(K,V)`

`len`, `is_empty`, `capacity`, `reserve`, `get`, `set`, `contains_key`,
`remove`, `clear`, **[A]** `keys() -> list(K)` (all key types), `values() ->
list(V)`, `get_or(key, default: V) -> V`.

`get`/`remove` on a missing key use the established runtime key/bounds error
behavior; `get_or` is the non-failing fallback. Key iteration order is
unspecified. `keys()` rebuilds string keys as handles and returns other keys in
their key layout; `values()` retains managed values into the returned list.

## `array(T,N)`

`len`, `at`, `set`, `first`, `last`, `fill`, **[A]** `to_list() -> list(T)`,
`contains(v) -> bool`, `reverse()`, `sort()` (orderable elements; same Vut
ordering contract as `list.sort`).

`array.to_list` builds a new `list(T)`; the array and the list are independent
(`to_list` copies/retains elements). `contains` compares element bytes, matching
`list.contains` (managed elements compare by identity). Array indexing traps on
out-of-range, as before. `array` has no higher-order methods; convert with
`to_list()` first.

## `bytes`

`len`, `is_empty`, `capacity`, `reserve`, `at`, `set`, `first`, `last`,
`slice`, `byte_at`, `clear`, `to_list`, `to_str`, plus `bytes()` and
`bytes.from_list(list(u8))`,
**[A]** `push(u8)`, `extend(bytes)`, `truncate(n)`, `resize(n, u8)`,
`find(needle: bytes) -> int`, `starts_with`/`ends_with(bytes) -> bool`,
`compare(bytes) -> int`, `to_hex() -> str`,
`bytes.from_hex(text: str) -> result(bytes, HexError)`, and the explicit-endian
family `read_{u16,i16,u32,i32,u64,i64}_{le,be}(offset)` /
`write_{u16,i16,u32,i32,u64,i64}_{le,be}(offset, value)`.

- `read_i32`/`read_i64` are **removed**; use the `_le`/`_be` variants (the byte
  order is always explicit).
- `find` returns the first occurrence byte offset or `-1`.
- `write_*` only writes within the current length; an out-of-range write is a
  no-op (same as `set`).
- `from_hex` accepts even-length hexadecimal; `HexError.index` is the byte
  index of the first invalid digit, or `text.byte_len()` when the digit count
  is odd. `to_hex` emits lowercase hexadecimal.

## `str`

`byte_len`, `char_len`, `is_empty`, `contains`, `starts_with`, `ends_with`,
`trim`, `trim_start`, `trim_end`, `to_lower`, `to_upper`, `replace`,
`to_bytes`, `to_i64`, `to_f64`, `find`, `split`, `substring`,
**[A]** `lines`, `split_whitespace`, `chars`, `char_at`, `repeat`, `pad_left`,
`pad_right`, `strip_prefix`, `strip_suffix`, `rfind`, `compare`, `to_int`,
`to_float`.

- `find`, `rfind`, `substring` use **byte** indices.
- `char_at`, `chars`, and the `pad_*` width use **Unicode scalar** indices.
  `char_at(i)` returns one scalar; out-of-range or negative yields `""`.
- `lines` uses Rust `str::lines` semantics (`"a\n"` → `["a"]`, `"a\n\n"` →
  `["a", ""]`, `""` → `[]`).
- `split_whitespace` splits on Unicode White_Space, collapsing runs.
- `repeat(n)`: `n <= 0` yields `""`; an overflowing result yields `""`.
- `pad_left`/`pad_right`: `width` counts scalars; `fill` must be exactly one
  Unicode scalar or the string is returned unchanged.
- `strip_prefix`/`strip_suffix` return the original string when absent.
- `compare` returns `-1`/`0`/`1` by lexicographic Unicode-scalar order.
- `to_int`/`to_float` are aliases of `to_i64`/`to_f64`.

## Numeric scalars (`int`, `float`, sized ints, `f64`)

**[A]** `abs()`, `min(other)`, `max(other)`, `clamp(low, high)` for `int` and
`float`; `int.pow(exponent: int) -> int`, `float.pow(exponent: float) -> float`;
`float.floor()`, `ceil()`, `round()`, `trunc()`, `sqrt()`,
`to_int() -> int` (NaN → 0, saturating), `is_nan() -> bool`,
`is_finite() -> bool`.

- Integer overflow saturates for `pow`; `abs` wraps.
- `f32` math methods are not available yet.
- `min`/`max` ignore NaN (Rust `f64::min`/`max`).

## `bool`

**[A]** `to_str() -> str` (`"true"` / `"false"`).

## `result(T,E)`

**[A]** `is_ok() -> bool`, `is_err() -> bool`, `unwrap_or(default: T) -> T`.
`unwrap_or` returns the `ok` payload, or `default` when the result is `err`.
Compiled to a tag test and branch; no allocation.
