---
title: Control flow
description: 'Express decisions with boolean conditions and indentation-based blocks.'
section: Language
---

## Conditions

Conditions must have type `bool`. Integers, strings, and collections are not automatically treated as booleans.

```vut
active = true
if active:
  out("Ready")
else:
  out("Not ready")
```

## Multiple branches

Use `elif` for additional conditions:

```vut
score = 85
if score >= 90:
  out("A")
elif score >= 80:
  out("B")
else:
  out("Keep going")
```

## Logical operators

Vut uses `and`, `or`, and `not`.

```vut
age = 20
active = true
if age >= 18 and active:
  out("Welcome")
```

## Loops

Vut uses `for` for loops. It intentionally does not provide a `while` keyword. Use `break` to leave a loop and `continue` to advance to its next iteration.

See [Collections](/docs/language/collections/) for the list literal syntax.
