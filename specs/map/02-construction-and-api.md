# Construction and API

Map literal syntax:

```vut
scores = map(("alice", 10), ("bob", 20))
empty: map(str, i32) = map()
```

Each entry is a parenthesized `(key, value)` pair. This avoids `{}` and `[]` and does not conflict with `@(...)` list literals or `array(...)`.

Empty `map()` requires an expected `map(K, V)` type. The compiler must not infer `dyn`.

Required methods:

```vut
m.len()
m.is_empty()
m.capacity()
m.reserve(capacity)
m.get(key)
m.set(key, value)
m.contains_key(key)
m.remove(key)
m.clear()
```

`set` inserts missing keys and replaces existing keys. `get` and `remove` report absence through the optional/result model when that representation is fully lowered; current native MVP returns the value storage and a runtime presence flag internally.

Indexing syntax is not supported:

```vut
m[key] # invalid
```
