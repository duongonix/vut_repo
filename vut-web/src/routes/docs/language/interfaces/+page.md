---
title: Interfaces
description: 'Describe behavior with structural method compatibility.'
section: Language
order: 7
---

## Declare behavior

```vut
interface Animal:
  speak() -> str

data Dog:
  name: str

fn Dog.speak() -> str:
  "Woof"
```

`Dog` satisfies `Animal` by providing its required public method. No `impl`, `implements`, or class inheritance declaration is needed.

## Compatibility

Methods must have matching names, parameter counts and order, compatible parameter and return types, and be available at the use site. A missing method or incompatible signature is a compile-time error.

## State belongs in data

Interfaces declare behavior, not fields. Interface method signatures omit `self`; instance behavior has an implicit receiver. Put stored state in a `data` declaration.
