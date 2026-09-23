# Result Construction and Patterns

Result constructors are contextual:

```vut
fn parse() -> result[int, str]:
  ok(42)

fn fail() -> result[int, str]:
  err("invalid")
```

`ok(value)` requires an expected `result[T, E]` type and checks `value` against
`T`. `err(error)` requires an expected `result[T, E]` type and checks `error`
against `E`.

This is invalid because the error type cannot be inferred:

```vut
value = ok(10)
```

Result values are matched with result patterns:

```vut
message = match load():
  ok(value): "loaded $value"
  err(error): "failed $error"
```

Both `ok(...)` and `err(...)` arms are required. Result patterns bind their
payload only inside that arm. Enum variant patterns and result patterns are not
interchangeable.

