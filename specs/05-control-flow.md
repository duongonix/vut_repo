# Vut Control Flow

## 1. Purpose

This document defines Vut control-flow constructs.

The core constructs are:

- `if`
- `elif`
- `else`
- `match`
- `for`
- `break`
- `continue`
- `return`

Vut intentionally does not provide `while`.

All loops use `for`.

---

## 2. Boolean Conditions

Control-flow conditions must have type:

```text
bool
```

Valid:

```vut
if active:
  print("active")
```

Invalid:

```vut
if 10:
  print("invalid")
```

Vut does not use general truthiness.

Integers, strings, lists and arbitrary objects are not automatically interpreted as booleans.

---

## 3. `if`

Basic syntax:

```vut
if condition:
  ...
```

Example:

```vut
if age >= 18:
  print("adult")
```

A colon begins the block.

The body is defined by indentation.

---

## 4. `else`

```vut
if active:
  print("active")
else:
  print("inactive")
```

`else` does not have a condition.

---

## 5. `elif`

Additional conditions use `elif`.

```vut
if score >= 90:
  print("A")
elif score >= 80:
  print("B")
elif score >= 70:
  print("C")
else:
  print("D")
```

Vut uses `elif`, not:

```text
else if
elseif
```

---

## 6. Nested Conditions

Conditions may be nested.

```vut
if active:
  if admin:
    print("admin")
  else:
    print("user")
```

Indentation determines nesting.

---

## 7. Logical Conditions

Vut uses:

```text
and
or
not
```

Example:

```vut
if age >= 18 and active:
  login()
```

```vut
if admin or owner:
  allow()
```

```vut
if not banned:
  allow()
```

Do not use:

```text
&&
||
!
```

for boolean logic.

---

## 8. `if` as an Expression

`if` may produce a value.

```vut
max = if a > b:
  a
else:
  b
```

This enables expression-oriented code without requiring a ternary operator.

Example:

```vut
status = if active:
  "online"
else:
  "offline"
```

---

## 9. `if` Expression Result

When an `if` expression is used where a statically known type is required, its resulting value must satisfy the surrounding type requirements.

Example:

```vut
value: int = if active:
  10
else:
  20
```

is valid.

Exact branch type-unification rules must be handled consistently by the type-system specification.

The compiler must not silently introduce `dyn` to resolve incompatible branches.

---

## 10. `match`

`match` selects a branch based on a value.

Example:

```vut
text = match status:
  pending: "Waiting"
  running: "Running"
  done: "Done"
```

`match` may be used as an expression.

### 10.1 Multi-statement arm bodies

An arm body may be a single inline expression or an indented block:

```vut
match result:
  ok(value):
    name = value.name
    "Hello $name"
  err(error):
    error.message
```

A block arm's value is its final expression; if the final statement is not an
expression the arm value is `void`. This allows a nested `match` as a multi-line
arm body:

```vut
match outcome:
  ok(value):
    match value.kind:
      user: "user"
      team: "team"
  err(error): "error"
```

A block may likewise be used as the right-hand side of an assignment:

```vut
total: int =
  a = 10
  b = 20
  a + b
```

Block expressions are not accepted inside parentheses, template interpolation, or
loop/condition headers.

---

## 11. Enum Matching

Enums are a primary use case.

```vut
enum Status:
  pending
  running
  done
```

Then:

```vut
text = match status:
  pending: "Waiting"
  running: "Running"
  done: "Done"
```

A bare identifier pattern is treated as an enum variant when the matched type is
an enum with a variant of that name; otherwise it is a binding.

The compiler performs exhaustiveness checking when the matched type has a finite
known set of variants.

---

## 12. Match Exhaustiveness

For a finite enum:

```vut
enum Status:
  pending
  running
  done
```

A match omitting `done` is a compile error (`E5009`) when exhaustive matching is
required.

The diagnostic identifies the missing variants.

Reachability is also checked: an arm already covered by earlier unguarded arms
is reported as an unreachable arm warning (`W5004`).

---

## 13. Patterns

Vut supports a full pattern grammar.

```text
_                       wildcard, ignores the value
name                    binding, or an enum variant when one matches
0                       integer literal
1.5                     float literal
"text"                  string literal
true / false            boolean literal
null                    null (optional) - reserved
variant                 payloadless enum variant
variant(field = p)      enum variant with a named sub-pattern
variant(p)              single-field positional shorthand
ok(p) / err(p)          result pattern with a sub-pattern
p1 or p2                or-pattern (alternatives bind the same names)
0..10 / 0..=10          integer range
@[p1, p2] / @[]         list pattern
(p)                     grouped pattern
p if condition:         guard on an arm
```

Examples:

```vut
match shape:
  point: "point"
  circle(radius = r) if r > 0.0: "circle"
  rect(width = w, height = h): "rect"

match value:
  0: "zero"
  1 or 2: "small"
  3..=9: "medium"
  _: "other"

match option:
  none: "none"
  some(value = v): v
```

Rules:

```text
enum variant patterns require an enum scrutinee
result ok/err patterns require a result scrutinee
literal patterns require a matching scrutinee type
guards must be bool
or-pattern alternatives must bind identical names and types
a variant pattern binds every field; use `_` to ignore one
```

List patterns are reserved and not yet implemented; a list match must use a
wildcard or guard fallback.

---

# Loops

## 14. One Loop Construct

Vut has one loop keyword:

```text
for
```

Vut does not have:

```text
while
```

Do not add `while` as syntactic sugar.

The four core loop forms are:

```text
for:
for condition:
for value in iterable:
for value, index in iterable:
```

---

## 15. Infinite Loop

An infinite loop uses:

```vut
for:
  work()
```

This is the canonical infinite-loop syntax.

There is no need for:

```text
while true
loop
forever
```

---

## 16. Conditional Loop

A conditional loop places a boolean expression directly after `for`.

```vut
i = 0

for i < 10:
  print(i)
  i = i + 1
```

This replaces the traditional `while` construct.

The condition must have type `bool`.

Invalid:

```vut
for 10:
  print("invalid")
```

Diagnostic concept:

```text
error[E5003]: loop condition must be `bool`
  --> src/main.vut:1:5
   |
1  | for 10:
   |     ^^ found `int`
   |
   = expected: bool
   = found:    int
```

---

## 17. Iterable Loop

Iterate over values using:

```vut
for item in items:
  print(item)
```

The expression after `in` must be iterable.

The compiler determines the type of `item` from the iterable's element type.

---

## 18. Literal Iterable

Collection literals may be iterated directly.

```vut
for value in @[10, 20, 30]:
  print(value)
```

The loop variable is inferred as `int`.

---

## 19. Value and Index

Vut supports two loop bindings.

The order is always:

```text
value, index
```

Example:

```vut
for value, index in items:
  print(value)
  print(index)
```

For:

```vut
@[10, 20, 30]
```

the logical sequence is:

```text
value = 10, index = 0
value = 20, index = 1
value = 30, index = 2
```

The variable names themselves are arbitrary.

```vut
for user, i in users:
  print(user)
  print(i)
```

The first binding is still the value.

The second binding is still the index.

---

## 20. Maximum Loop Bindings

Iterable `for` supports:

```vut
for value in items:
```

or:

```vut
for value, index in items:
```

It does not support arbitrary positional bindings.

Invalid:

```vut
for value, index, other in items:
  ...
```

The compiler must produce a clear diagnostic.

---

## 21. Range

Ranges are iterable expressions.

Exclusive range:

```vut
0..10
```

represents values:

```text
0
1
2
3
4
5
6
7
8
9
```

Usage:

```vut
for i in 0..10:
  print(i)
```

The upper bound is excluded.

---

## 22. Inclusive Range

Inclusive ranges use:

```vut
0..=10
```

Usage:

```vut
for i in 0..=10:
  print(i)
```

The upper bound is included.

Logical values:

```text
0
1
2
3
4
5
6
7
8
9
10
```

---

## 23. Range Is an Expression

Range syntax is not special to the `for` grammar.

Conceptually:

```vut
range = 0..10
```

produces an iterable range value if the surrounding language semantics permit storing ranges.

The `for` loop simply consumes an iterable.

This design prevents separate loop-specific range semantics.

---

## 24. `break`

`break` exits the nearest enclosing loop.

```vut
for:
  if done:
    break
```

Nested example:

```vut
for row in rows:
  for value in row:
    if value == target:
      break
```

The `break` affects the nearest loop.

Labeled break syntax is not currently defined.

---

## 25. `continue`

`continue` skips the remainder of the current iteration and proceeds to the next iteration.

```vut
for item in items:
  if item == null:
    continue

  process(item)
```

It applies to the nearest enclosing loop.

---

## 26. Invalid `break`

Using `break` outside a loop is a compile error.

Invalid:

```vut
fn test():
  break
```

The compiler must identify the invalid `break`.

---

## 27. Invalid `continue`

Using `continue` outside a loop is a compile error.

Invalid:

```vut
fn test():
  continue
```

---

## 28. `return` and Control Flow

`return` exits the current function or method.

```vut
fn find(items: list[int], target: int) -> int:
  for value, index in items:
    if value == target:
      return index

  return -1
```

`return` does not merely exit a loop.

It exits the enclosing function.

---

## 29. Infinite Loop and Return

An infinite loop may terminate through `return`.

```vut
fn wait_until_ready() -> int:
  for:
    if ready():
      return 1
```

The compiler's control-flow analysis should recognize non-fallthrough loops when possible.

---

## 30. Nested Loops

Loops may be nested.

```vut
for row in rows:
  for value in row:
    print(value)
```

Each loop has its own bindings.

Normal lexical scoping rules apply.

---

## 31. Loop Binding Scope

Loop bindings belong to the loop's body scope.

```vut
for user in users:
  print(user)
```

`user` is the current element inside the loop.

The exact availability of the binding after the loop is governed by lexical-scope rules and must be implemented consistently.

The compiler should avoid leaking temporary iteration bindings unexpectedly.

---

## 32. Iterable Requirements

The expression used in:

```vut
for value in expression:
```

must support Vut's iterable protocol or be a built-in iterable type.

Expected iterable examples include:

- list
- range
- string if explicitly specified by the standard library
- map if explicitly specified by the standard library
- custom iterable types once the iterable interface/protocol is defined

The compiler must not assume arbitrary values are iterable.

---

## 33. Index Semantics

For indexed iteration:

```vut
for value, index in items:
```

`index` represents the iteration index provided by the iterable.

For normal lists and ranges, indexing begins at zero unless the specific iterable contract defines otherwise.

The built-in list contract uses zero-based indexes.

---

## 34. No C-Style `for`

Vut does not use C-style loops.

Do not introduce:

```text
for (i = 0; i < 10; i++)
```

The Vut equivalent is:

```vut
for i in 0..10:
  ...
```

or, when a mutable condition is actually required:

```vut
i = 0

for i < 10:
  ...
  i = i + 1
```

---

## 35. No `while`

The following is not valid Vut:

```text
while condition:
```

Use:

```vut
for condition:
  ...
```

Infinite `while true` becomes:

```vut
for:
  ...
```

This rule is intentional and must not be treated as a missing feature.

---

## 36. No Truthiness

These constructs must not be accepted solely through implicit truthiness:

```vut
if 10:
  ...

for "hello":
  ...
```

Conditions require `bool`.

Users must explicitly create boolean expressions.

```vut
if count > 0:
  ...
```

---

## 37. Control-Flow Diagnostics

Control-flow errors should provide precise diagnostics.

Examples include:

- non-boolean condition
- invalid indentation
- missing `:`
- invalid loop binding count
- non-iterable expression
- `break` outside loop
- `continue` outside loop
- incompatible return
- incomplete enum match

Example:

```text
error[E0102]: expected `:`
  --> src/main.vut:3:12
   |
3  | if age > 18
   |            ^ expected `:` after condition
   |
   = help:
       if age > 18:
```

---

## 38. Control-Flow Grammar Direction

The intended loop grammar is conceptually:

```text
for_statement :=
    "for" ":" block
  | "for" expression ":" block
  | "for" identifier "in" expression ":" block
  | "for" identifier "," identifier "in" expression ":" block
```

The formal parser grammar is defined in:

```text
specs/21-grammar.md
```

This section defines language intent rather than parser implementation.

---

## 39. Control-Flow Principles

Vut control flow follows these principles:

1. Conditions are statically typed as `bool`.
2. `if` may be a statement-like construct or produce a value.
3. `match` provides structured branching.
4. Enum matches should support exhaustiveness checking.
5. All loops use `for`.
6. Vut has no `while`.
7. `for:` means infinite loop.
8. `for condition:` means conditional loop.
9. `for value in iterable:` means iterable loop.
10. `for value, index in iterable:` provides value then index.
11. Ranges are iterable expressions.
12. `break` and `continue` affect the nearest loop.
13. `return` exits the current function.
14. Control-flow syntax must remain indentation-based.
15. Invalid control flow must produce precise source diagnostics.

This document is normative for the core control-flow behavior of Vut.