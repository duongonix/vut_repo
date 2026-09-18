# Vut Optional Values

Status: normative for 0.4. This document finalizes the optional (`T?`)
narrowing/unwrapping semantics that were previously deferred, and the
`map.get`/`map.remove` result model.

## 1. Type

`T?` is the optional form of `T`.

```vut
nickname: str? = null
age: int? = 30
```

* `null` is valid only where an optional `T?` is expected.
* A value of type `T` is compatible with `T?` (present).
* `T?` is **not** compatible with `T`; using an optional where `T` is
  required is an error (`E1007`).
* `null` assigned to a non-optional type is an error (`E1006`).

## 2. Representation

The representation is not visible to source.

* If `T` is a managed handle (`str`, `bytes`, `list`, `map`, `vutcon`,
  `future`, `resource`, interfaces, `dyn`), `T?` is a nullable handle: a zero
  handle is absent, a non-zero handle is present. Runtime retain/release
  entry points are null-safe.
* Otherwise `T?` is a tagged aggregate: a presence discriminant plus `T`. A
  zero discriminant is absent.

The compiler may use niche/null optimization where valid.

## 3. Narrowing

Narrowing refines the static type of an optional place within a control-flow
region. The storage is unchanged; the compiler inserts the unwrap.

### 3.1 Comparisons

`x == null` and `x != null` are valid when `x` is `T?`. The result is `bool`.
No other comparison between `T?` and a non-optional value is allowed.

### 3.2 `if` narrowing

```vut
if value != null:
  use(value)          # value: T
else:
  out("absent")       # value: T?

if value == null:
  out("absent")       # value absent
else:
  use(value)          # value: T
```

### 3.3 Guard-clause narrowing

If the absent branch terminates the enclosing flow (`return`, `break`,
`continue`, or `?`-propagating error), the code after the `if` sees the
present type.

```vut
for item in items:
  if item == null:
    continue
  process(item)       # item: T
```

### 3.4 `match` patterns

`null` is a valid pattern for an optional subject. The `null` arm sees the
absent value; every other arm sees the present type `T`.

```vut
match scores.get("ann"):
  null: out("missing")
  value: out("got $value")   # value: int
```

Exhaustiveness requires `null` or `_` for an optional subject.

## 4. No silent unwrap

There is no implicit conversion from `T?` to `T` and no unchecked `!`
operator. A value is usable as `T` only after narrowing (§3) or through an
explicit non-failing helper such as `map.get_or(key, default)`.

## 5. `map.get` / `map.remove`

```text
get(key)    -> V?
remove(key) -> V?
```

Absence is reported as `null`; the operations do not trap on a missing key.
`get_or(key, default) -> V` remains the non-failing fallback. `set` and
`contains_key` are unchanged. `list`/`bytes`/`array` index operations keep
their existing trapping bounds behavior; only map lookup/removal reports
absence through the optional model.

## 6. Errors

```text
E1006  null assigned to a non-optional type
E1007  invalid optional operation (unsafe unwrap / optional misuse)
```

## 7. Ownership

* Narrowing does not copy or retain; it only changes how the existing storage
  is read.
* An optional managed value owns its handle when present and nothing when
  absent; dropping it releases a present handle exactly once.
* Partial moves, early returns, and error paths follow the normal memory
  model.
