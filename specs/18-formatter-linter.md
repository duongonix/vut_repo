
# Vut Formatter and Linter

The production formatter operates on a lossless syntax tree. Every input byte
belongs to a syntax token, including whitespace and comments, and the unmodified
tree must reproduce the input byte-for-byte. Canonical formatting is a separate
printing pass; it must not reconstruct source from the semantic AST or modify
content embedded in strings, templates, or comments.

## 1. Purpose

This document defines the behavior and boundaries of:

```text
vpm fmt
vpm lint
```

The formatter standardizes Vut source layout.

The linter detects suspicious or undesirable code that is still syntactically and semantically valid.

---

## 2. Formatter Philosophy

Vut has one canonical style.

The formatter should minimize stylistic configuration.

Users should not need to debate:

* indentation width
* comma style
* spacing
* operator spacing
* block layout

The canonical formatter determines these consistently.

---

## 3. Command

Format project:

```text
vpm fmt
```

The formatter should discover Vut source in appropriate project locations.

Typical:

```text
src/
tests/
```

---

## 4. Indentation

Canonical indentation:

```text
2 spaces
```

Example:

```vut
if active:
  print("active")

  if admin:
    print("admin")
```

Do not emit tabs for indentation.

---

## 5. Block Formatting

Blocks begin after `:`.

Example:

```vut
if active:
  login()
else:
  logout()
```

The formatter must preserve indentation-based semantics.

---

## 6. Semicolons

Vut does not require semicolons.

The formatter must not introduce semicolons.

---

## 7. Operator Spacing

Binary operators use spaces.

```vut
a + b
a == b
age >= 18
active and admin
```

Not:

```text
a+b
a==b
```

---

## 8. Assignment Spacing

Use:

```vut
name = "Ha"
age: int = 20
```

---

## 9. Commas

Inline comma-separated values use one space after commas.

```vut
@(1, 2, 3)
```

```vut
fn add(a: int, b: int) -> int:
```

---

## 10. Multi-Line Calls

Long calls may be expanded.

```vut
user = User(
  name = "Ha",
  age = 20,
  active = true
)
```

Closing parenthesis aligns with the beginning of the expression.

---

## 11. `data` Formatting

Canonical:

```vut
data User:
  name: str
  age: int
  active: bool = true
```

No commas are inserted between indentation-based field declarations.

---

## 12. Interface Formatting

```vut
interface Reader:
  read(size: int) -> bytes
```

Composition:

```vut
interface ReadWriter: Reader, Writer:
  close()
```

---

## 13. Method Formatting

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Preserve the required `fn` keyword.

Do not add explicit `self`.

---

## 13a. Async Formatting

Preserve the `async` modifier and print one space before `fn`:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

Preserve `await` as a prefix with one space before its operand:

```vut
value = await operation()
```

The formatter must not rewrite `await` into a postfix form, remove it, or merge
`await` and `?`.

---

## 14. `if` Formatting

```vut
if age >= 18:
  print("adult")
elif age >= 13:
  print("teen")
else:
  print("child")
```

---

## 15. `for` Formatting

Infinite:

```vut
for:
  work()
```

Conditional:

```vut
for running:
  work()
```

Iterable:

```vut
for item in items:
  process(item)
```

Value/index:

```vut
for value, index in items:
  process(value, index)
```

Range:

```vut
for i in 0..10:
  print(i)
```

---

## 16. Imports

Canonical:

```vut
import math
import math as m
import math at add, plus
```

The formatter must not convert one valid import style into another with different binding semantics.

---

## 17. Import Ordering

The formatter may group/sort imports deterministically.

Sorting must not alter program semantics or create collisions.

Exact grouping policy should remain simple.

---

## 18. List Formatting

Short:

```vut
numbers = @(1, 2, 3)
```

Long:

```vut
numbers = @(
  10,
  20,
  30
)
```

---

## 19. Comments

Single-line comments:

```vut
# comment
```

Inline comments should retain readable spacing:

```vut
name = "Ha" # comment
```

Multi-line comments retain:

```vut
##
comment
##
```

The formatter should preserve comment association as reliably as possible.

---

## 21. Idempotency

Formatting must be idempotent.

Running:

```text
vpm fmt
```

twice must not continue changing already formatted files.

Conceptually:

```text
format(format(source)) == format(source)
```

---

## 22. Semantic Preservation

Formatting must never change program semantics.

If the formatter cannot safely understand malformed source, it should report a diagnostic rather than guessing destructive edits.

---

## 23. Check Mode

CI should eventually support checking formatting without modifying files.

Conceptually:

```text
vpm fmt --check
```

Exact flag may be finalized with VPM CLI implementation.

---

## 24. Linter Purpose

`vpm lint` reports code that is legal but potentially:

* suspicious
* error-prone
* inefficient
* unclear
* redundant

Lint findings are not compiler syntax/type errors.

---

## 25. Compiler Errors vs Lints

Compiler:

```text
invalid type
unknown symbol
invalid import
```

Linter:

```text
unused variable
unused import
redundant condition
unreachable branch
```

Do not move required correctness rules into optional lint rules.

---

## 26. Warning Codes

Lint warnings use stable:

```text
W####
```

codes where practical.

Example:

```text
warning[W2001]: unused variable `value`
```

---

## 27. Unused Variables

Example:

```vut
value = calculate()
```

when `value` is never used.

Potential diagnostic:

```text
warning[W2001]: unused variable `value`
```

Identifiers intentionally beginning with `_` may suppress unused-variable warnings where appropriate.

Example:

```vut
_result = calculate()
```

Privacy and unused-binding semantics must not be confused; `_` remains the language privacy marker for module-level declarations.

---

## 28. Unused Imports

```vut
import math
```

when `math` is never referenced may generate a warning.

The formatter must not automatically remove imports unless explicitly operating in a fix mode.

---

## 29. Unreachable Code

Example:

```vut
fn run():
  return

  print("never")
```

The linter/compiler control-flow layer may warn about unreachable code.

---

## 30. Constant Conditions

Example:

```vut
if true:
  ...
```

may produce a lint warning when it appears accidental.

Do not warn when such a construct has a legitimate compile-time purpose defined by the language.

---

## 31. Redundant Boolean Expressions

Example:

```vut
if active == true:
```

may be linted toward:

```vut
if active:
```

when semantics are identical.

---

## 32. Shadowing

Variable shadowing policy has not been fully finalized.

The linter may eventually warn on suspicious shadowing after language scoping rules are finalized.

Do not reject shadowing merely because a linter rule is planned.

---

## 33. Automatic Fixes

Some lints may offer safe automatic fixes.

Conceptually:

```text
vpm lint --fix
```

A fix must only be applied when semantics can be preserved reliably.

Avoid speculative automatic transformations.

---

## 34. Machine Output

Formatter/linter should support tooling integration.

Lint diagnostics should use the same structured diagnostic infrastructure as the compiler where practical.

---

## 35. Reuse Compiler Frontend

Formatter and linter should reuse:

```text
lexer
parser
AST
source manager
diagnostic model
```

Do not implement an independent Vut parser.

---

## 36. Performance

Formatter and linter should support large projects efficiently.

Project file discovery should respect ignore rules for:

```text
build/
.git/
VPM caches
```

and other non-source directories.

---

## 37. Formatter/Linter Principles

1. Vut has one canonical formatting style.
2. Canonical indentation is two spaces.
3. Formatter is deterministic and idempotent.
4. Formatting never changes semantics.
5. Formatter reuses the compiler frontend.
6. Compiler errors and lint warnings are distinct.
7. Lints use stable warning codes where appropriate.
8. Automatic fixes must be conservative.
9. Tooling should support CI/machine-readable workflows.
10. Avoid excessive style configuration.

---


