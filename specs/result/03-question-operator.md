# Question Operator

The postfix `?` operator unwraps a successful result or propagates an error:

```vut
fn load_name() -> result(str, LoadError):
  user = load_user()?
  ok(user.name)
```

The operand must have type `result(T, E)`. The expression type of `value?` is
`T`.

The current function must return `result(_, E2)` where `E` is compatible with
`E2`. Vut performs no implicit error conversion.

Invalid examples:

```vut
fn bad() -> int:
  load_user()?

fn mismatch() -> result(User, OtherError):
  load_user()?
```

`?` lowers to an explicit branch in MIR. The `err` branch constructs and returns
the current function's result error state; the `ok` branch extracts the success
payload and continues.

`await` and `?` are independent. `await` binds tighter than `?`, so:

```vut
data = await read_async()?
```

parses as:

```text
(await read_async())?
```

where `read_async` is an `async fn` returning `result(T, E)`. The `await`
produces `result(T, E)`; `?` then propagates or unwraps it. See:

```text
specs/async/02-type-system.md
```

