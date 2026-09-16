# Result Type System and Memory

`result(T, E)` is invariant in both payload types. A value of
`result(A, B)` is compatible with `result(C, D)` only when `A` is compatible
with `C` and `B` is compatible with `D`.

Result may be nested and may appear inside other core containers:

```vut
items: list(result(int, str)) = @(ok(1))
nested: result(result(int, str), str) = ok(ok(1))
```

Native representation is a tagged aggregate:

- tag `0` means `success`
- tag `1` means `error`
- only the active payload is semantically live

Result is not a heap object by default. Codegen stores it as a native aggregate
with tag and payload slots. Ownership operations retain or release only the
active payload selected by the tag.

FFI v1 rejects `result(T, E)` in extern function parameters and return types.
Future ABI-safe result structs must be specified explicitly before FFI accepts
them.

