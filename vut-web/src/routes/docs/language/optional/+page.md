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

The core type-system specification leaves the complete optional narrowing and unwrapping surface to a later definition. This guide intentionally does not invent an optional-chain operator or promise unchecked access.

## Not result propagation

The postfix expression operator `value?` documented for [Result](/docs/language/result/) requires a result operand. Do not infer that it unwraps optionals merely because `T?` uses the same punctuation.
