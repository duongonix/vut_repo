---
title: Loops
description: 'One loop keyword for conditions, collections, and ranges.'
section: Language
order: 4
---

## Iterate over values

```vut
fn main():
  for value, index in @[10, 20, 30]:
    out("$index: $value")
```

The order is **value, index**, with a zero-based index for normal lists and ranges. One binding is also valid: `for value in items:`.

## Conditional and infinite loops

```vut
fn main():
  count = 0
  for count < 3:
    out("$count")
    count = count + 1

  for:
    break
```

A conditional loop requires `bool`. `for:` is the infinite-loop form. There is no `while` or C-style `for`.

## Ranges

```vut
fn main():
  for value in 0..10:
    if value == 3:
      continue
    if value == 8:
      break
    out("$value")
```

`break` exits the nearest loop. `continue` skips its current iteration. Both are errors outside a loop. `return` exits the enclosing function, not just the loop.
