---
title: Collections
description: 'Work with typed collections using Vut’s own literal syntax.'
section: Language
---

## List literals

A list literal uses `@[]`. Square brackets without `@` construct an array.

```vut
numbers = @[1, 2, 3]
names = @["Ha", "Nam", "Lan"]
```

## Element types

Lists are homogeneous by default. Declare the element type using square brackets:

```vut
numbers: list[int] = @[1, 2, 3]
```

An incompatible element is a type error.

Use [Lists and arrays](/docs/language/lists-arrays/) for mutation, iteration, and higher-order operations; [Maps](/docs/language/maps/) for keyed lookup; and [Strings and bytes](/docs/language/strings-bytes/) for text and binary data.

## Dynamic elements

Mixed element types require an explicit dynamic element type:

```vut
values: list[dyn] = @[1, "hello", true]
```

## Named data

For structured values, declare named `data`. Vut does not have tuples or anonymous record literals.

```vut
data Point:
  x: int
  y: int

point = Point(x: 10, y: 20)
```
