# Type system and memory

Map literals are homogeneous:

- every key has type `K`;
- every value has type `V`;
- mixed types require explicit `dyn` under existing dyn rules.

Key types must support equality and hashing. The type checker owns this capability rule through centralized `supports_hash` / equality metadata; parser and runtime must not carry scattered key allow-lists.

MVP key types:

- integers and numeric primitives;
- `bool`;
- `str` once content hashing/equality is used by the active ABI.

Managed values and nested maps are valid type-system shapes:

```vut
map(str, map(str, i32))
list(map(str, User))
result(map(str, User), Error)
```

`map(K,V)` is managed and `needs_drop = true`. Maps inside `data` fields participate in aggregate ownership metadata.

FFI v1 does not treat `map(K,V)` as C-safe directly. Extern parameters or returns containing map must be rejected unless an explicit C-compatible wrapper is introduced.
