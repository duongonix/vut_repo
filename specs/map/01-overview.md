# Map overview

`map[K, V]` is Vut's dynamic key/value collection, alongside `list[T]` and `array[T, N]`.

Canonical type syntax:

```vut
map[str, i32]
map[i32, User]
map[str, list[User]]
```

Do not use `Map<K,V>`, `Map(K,V)`, or `map(K,V)`.

Maps have value semantics at the language level. Native runtime storage is managed and reference-counted; mutation must not make an older user-visible value observe changes through an alias.

Map iteration order is not guaranteed and hash values are not public API.
