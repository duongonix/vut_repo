---
title: Optional values
description: 'Represent absence explicitly with the T? type.'
section: Language
order: 5
---

## Nullable values

An optional type is written `T?`. It accepts `null` or a compatible value of the underlying type.

```vut
nickname: str? = null
nickname = "Vut"
```

A normal non-optional value does not accept `null`: `name: str = null` is invalid.

## Access and narrowing

Testing a binding against `null` narrows it in the branch where the value is present:

```vut
fn describe(name: str?) -> str:
  if name == null:
    return "Anonymous"
  name.upper()
```

The early return removes the absent path, so the remaining code can use `name` as `str`. Likewise, `if name != null:` narrows the present branch; the `else` branch of `if name == null:` is present. A later assignment must still respect the binding's declared type and can invalidate what is known about its value.

## Matching absence

```vut
fn label(value: int?) -> str:
  match value:
    null: "missing"
    present: present.to_str()
```

A present binding receives the underlying value. Cover the absent case with `null` or a wildcard. Passing an optional to a non-optional parameter without proving presence is a type error (E1007); assigning `null` to a non-optional type is E1006.

## Collections and APIs

`map.get(key)` and `map.remove(key)` return `V?`: an absent key produces `null`, not a trap or a fabricated default. Use a presence test or `map.get_or(key, fallback)`.

An API can return `result[str?, Error]` when failure and absence are different states. First handle or propagate the result error, then handle the optional value. `env.get(name)` is an example.

## Not result propagation

The postfix expression operator `value?` documented for [Result](/docs/language/result/) requires a result operand. Do not infer that it unwraps optionals merely because `T?` uses the same punctuation.
