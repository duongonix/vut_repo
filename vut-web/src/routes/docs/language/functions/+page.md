---
title: Functions
description: 'Small, expressive functions with explicit parameters and return types.'
section: Language
---

<script>
  import ApiReference from '$lib/components/docs/ApiReference.svelte';
</script>

## Declare a function

Use `fn`, a parameter list, and a colon:

```vut
fn greet(name: str):
  out("Hello, $name!")
```

A function without a return annotation returns `void`; omitting the annotation does not request return-type inference.

## Return values

Write the return type after `->`. The final expression is returned automatically.

```vut
fn add(a: int, b: int) -> int:
  a + b
```

<ApiReference signature="fn add(a: int, b: int) -> int" parameters="a and b are the integer operands." returns="The sum of a and b." />

## Early returns

Use `return` to leave a function before its final expression:

```vut
fn divide(a: int, b: int) -> int:
  if b == 0:
    return 0
  a / b
```

All return paths must satisfy the declared return type.

## Call a function

Arguments use parentheses:

```vut
fn main():
  answer = add(20, 22)
  out("The answer is $answer")
```
