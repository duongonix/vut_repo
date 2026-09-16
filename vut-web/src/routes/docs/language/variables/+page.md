---
title: Variables
description: 'Declare values, use type inference, and keep your intentions clear.'
section: Language
---

## Overview

Variables are declared through assignment. Vut does not require a `let` or `var` keyword.

```vut
name = "Vut"
age = 20
active = true
```

## Type inference

The compiler infers a variable’s type from its initial assignment. Once inferred, the type stays fixed.

```vut
count = 10
count = 20
```

Both values are integers, so this reassignment is valid. Assigning a string to `count` would be a type error.

> **Note:** Type inference is not dynamic typing. Incompatible assignments do not silently turn a value into `dyn`.

## Type annotations

Use a colon to state the type explicitly:

```vut
name: str = "Vut"
age: int = 20
active: bool = true
```

Annotations are optional when the compiler can infer the type. An assigned value must always be compatible with the declared type.

## Constants

ALL-CAPS bindings are constants. They cannot be reassigned after initialization.

```vut
MAX_SIZE = 100
APP_NAME = "Vut"
```

There is no separate `const` keyword.

## Dynamic values

Dynamic behavior is explicit. Declare `dyn` when a variable needs to hold values of different types.

```vut
value: dyn = 10
value = "hello"
value = true
```

## Examples

Combine inferred values, explicit types, and template strings:

```vut title="main.vut"
fn main():
  name = "Vut"
  version: int = 1
  out("Welcome to $name, version $version")
```

Continue with [Types](/docs/language/types/) for the broader type system.
