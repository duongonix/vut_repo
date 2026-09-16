# Vut Errors and Diagnostics

## 1. Purpose

Compiler diagnostics are a first-class part of the Vut language experience.

Every diagnostic should answer as clearly as possible:

- what failed
- where it failed
- why it failed
- what was expected
- what was found
- where related declarations exist
- how the user may fix it when a reliable fix is known

Diagnostics must prioritize precision and readability.

---

## 2. Source Error Requirements

Every compiler error associated with source code must include:

1. severity
2. stable error code
3. short error title
4. file path
5. line
6. column
7. relevant source code
8. precise highlighted span
9. explanation
10. related source locations when useful
11. `expected` and `found` information when applicable
12. `help` when the compiler has a reliable correction

A compiler error must not merely say:

```text
type error
```

or:

```text
invalid code
```

when precise source information exists.

---

## 3. Canonical Error Layout

The canonical layout is:

```text
error[E1003]: type mismatch
  --> src/main.vut:12:9
   |
12 |   age = "20"
   |         ^^^^ expected `int`, found `str`
   |
   = expected: int
   = found:    str
   = help: assign an `int` value to `age`
```

Structure:

```text
error[CODE]: short title
  --> file:line:column
   |
NN | source
   | error span + explanation
   |
   = note: ...
   = help: ...
```

---

## 4. Severity Labels

Core diagnostic severities are:

```text
error
warning
note
help
```

`note` and `help` are normally attached to another diagnostic rather than treated as independent compilation failures.

Do not introduce excessive severity categories without a clear reason.

---

## 5. Error Codes

Compile errors use stable identifiers:

```text
E####
```

Warnings use:

```text
W####
```

Example:

```text
E1003
W1002
```

Error codes should remain stable across compatible compiler releases whenever practical.

The full registry is defined in:

```text
specs/22-error-codes.md
```

---

## 6. Error Code Categories

Initial category allocation:

```text
E0xxx  syntax / parser
E1xxx  type system
E2xxx  names / symbols
E3xxx  modules / imports
E4xxx  interfaces
E5xxx  control flow
E6xxx  functions / calls
E7xxx  data / enums
E8xxx  memory / safety / FFI
E9xxx  compiler internal failures
```

Warnings use corresponding `W` namespaces where appropriate.

Exact assignments belong to the error-code registry.

---

## 7. File, Line and Column

Source diagnostics must include:

```text
<file>:<line>:<column>
```

Example:

```text
src/user.vut:12:9
```

Line and column numbering are one-based in user-facing diagnostics.

The compiler must consistently map internal source offsets to user-facing positions.

---

## 8. Exact Source Span

Diagnostics must highlight the smallest useful span causing the error.

Example:

```vut
age: int = "20"
```

Correct:

```text
4 | age: int = "20"
  |            ^^^^ expected `int`, found `str`
```

Avoid unnecessarily highlighting:

```text
^^^^^^^^^^^^^^
```

when only `"20"` caused the problem.

---

## 9. Multi-Line Source Context

Diagnostics may include surrounding lines when they improve understanding.

Example:

```text
6 | user = User(
7 |   name = "Ha",
8 |   age = "20"
  |         ^^^^ expected `int`, found `str`
9 | )
```

The compiler should avoid printing unrelated large source regions.

---

## 10. Syntax Highlighting

Source code displayed inside diagnostics must use syntax highlighting when the output target supports color.

Highlight categories should include at minimum:

- keywords
- identifiers
- types
- strings
- numbers
- operators
- comments
- literals

Example source:

```vut
if age >= 18 and active:
```

should visually distinguish:

```text
if
age
>=
18
and
active
```

according to their syntax categories.

---

## 11. Diagnostic Colors

Recommended terminal roles:

```text
error              red / bold
warning            yellow
help               green
note               cyan or subdued
file location      cyan
line numbers       subdued
primary underline  red
secondary underline subdued/cyan
```

Exact ANSI codes are implementation details.

The same logical roles should be shared by all compiler diagnostics.

---

## 12. No-Color Fallback

Diagnostics must remain understandable without ANSI color.

Plain output must retain:

- labels
- source frames
- arrows
- underlines
- error codes
- note/help prefixes

Color must improve diagnostics, not be required to understand them.

---

## 13. Color Control

The compiler/tooling should support conventional color modes such as:

```text
auto
always
never
```

Exact CLI flags are defined by CLI specifications.

Default should detect terminal support automatically.

---

## 14. Related Locations

When an error depends on another declaration, show that location.

Primary location:

```text
-->
```

Related location:

```text
:::
```

Example:

```text
error[E1201]: argument type does not match parameter
  --> src/main.vut:14:11
   |
14 | save_user(10)
   |           ^^ expected `User`, found `int`
   |
:::
   |
7  | fn save_user(user: User):
   |              ---------- parameter declared here
```

---

## 15. Multiple Related Locations

A diagnostic may contain several related locations when required.

Examples include:

- duplicate declarations
- conflicting interface requirements
- import cycles
- previous constant declaration
- type inference origin
- duplicate fields

The diagnostic should keep the primary cause visually dominant.

---

## 16. Expected and Found

Type and syntax diagnostics should provide explicit expected/found information when useful.

Example:

```text
= expected: int
= found:    str
```

For signatures:

```text
= required: read(size: int) -> bytes
= found:    read(size: str) -> bytes
```

Alignment may be used to improve readability.

---

## 17. Help

Use `help` only when the compiler has a reasonably reliable suggestion.

Example typo:

```vut
pritn("hello")
```

Diagnostic:

```text
error[E2001]: unknown function `pritn`
  --> src/main.vut:3:1
   |
3  | pritn("hello")
   | ^^^^^ function not found
   |
   = help: did you mean `print`?
```

Do not generate misleading suggestions merely to always display a help line.

---

## 18. Notes

`note` provides additional explanation that is useful but not the primary failure.

Example:

```text
= note: `age` was inferred as `int` from its first assignment
```

Notes may explain:

- inference origins
- language rules
- dependency origin
- interface requirement
- previous declarations

---

## 19. Type Mismatch

Canonical example:

```text
error[E1003]: type mismatch
  --> src/main.vut:4:12
   |
4  | age: int = "20"
   |            ^^^^ expected `int`, found `str`
   |
   = expected: int
   = found:    str
```

If the expected type originates elsewhere, show the related location.

---

## 20. Fixed Inferred Variable Type

Example source:

```vut
age = 20
age = "twenty"
```

Diagnostic:

```text
error[E1003]: type mismatch
  --> src/main.vut:2:7
   |
2  | age = "twenty"
   |       ^^^^^^^^^ expected `int`, found `str`
   |
:::
   |
1  | age = 20
   | --- `age` was inferred as `int` here
```

---

## 21. Constant Reassignment

Example:

```vut
MAX_SIZE = 100
MAX_SIZE = 200
```

Diagnostic:

```text
error[E1104]: cannot assign to constant `MAX_SIZE`
  --> src/main.vut:5:1
   |
5  | MAX_SIZE = 200
   | ^^^^^^^^ constant cannot be reassigned
   |
:::
   |
2  | MAX_SIZE = 100
   | -------- constant declared here
```

---

## 22. Unknown Symbol

Example:

```vut
pritn("hello")
```

The compiler should perform bounded fuzzy matching against visible compatible names.

Suggestion quality matters more than suggestion quantity.

Do not dump a large unrelated symbol list.

---

## 23. Unknown Field or Method

Example:

```vut
user.naem
```

Diagnostic may suggest:

```text
name
```

Example:

```text
error[E2004]: unknown field `naem`
...
= help: did you mean `name`?
```

Method diagnostics follow the same principle.

---

## 24. Missing Module

Example:

```vut
import math.vector
```

Diagnostic:

```text
error[E3001]: module not found
  --> src/main.vut:1:8
   |
1  | import math.vector
   |        ^^^^^^^^^^^ module `math.vector` was not found
```

The compiler may show searched paths if doing so is useful.

Avoid displaying dozens of internal paths by default.

---

## 25. Private Symbol

Example:

```vut
import math at _fast_add
```

Diagnostic:

```text
error[E3004]: private symbol `_fast_add`
  --> src/main.vut:1:16
   |
1  | import math at _fast_add
   |                ^^^^^^^^^ private to module `math`
   |
:::
   |
21 | fn _fast_add(a: int, b: int) -> int:
   |    --------- declared private here
```

---

## 26. Circular Import

The diagnostic must display the dependency cycle.

Example:

```text
error[E3010]: circular module dependency

  a -> b -> c -> a
```

When possible, source locations of each import edge should be attached.

---

## 27. Interface Error

Example:

```text
error[E4102]: `Dog` does not satisfy interface `Animal`
  --> src/main.vut:10:6
   |
10 | play(Dog(name = "Milo"))
   |      ^^^^^^^^^^^^^^^^^^ `Dog` cannot be used as `Animal`
   |
   = missing method:
       speak() -> str
```

The interface declaration should be attached as a related location when useful.

---

## 28. Signature Mismatch

Example:

```text
error[E4103]: method signature does not satisfy interface
  --> src/dog.vut:8:1
   |
8  | fn Dog.speak(volume: int) -> str:
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = required: speak() -> str
   = found:    speak(int) -> str
```

---

## 29. Invalid Loop Condition

Example:

```vut
for 10:
  ...
```

Diagnostic:

```text
error[E5003]: loop condition must be `bool`
  --> src/main.vut:4:5
   |
4  | for 10:
   |     ^^ found `int`
   |
   = expected: bool
   = found:    int
```

---

## 30. Invalid Loop Bindings

Example:

```vut
for value, index, other in items:
```

Diagnostic:

```text
error[E5002]: invalid loop binding
  --> src/main.vut:7:5
   |
7  | for value, index, other in items:
   |     ^^^^^^^^^^^^^^^^^^^ Vut supports at most two loop bindings
   |
   = expected:
       for value in items:
       for value, index in items:
```

---

## 31. Missing Colon

Example:

```vut
if age > 18
  print(age)
```

Diagnostic:

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

Parser diagnostics should describe the Vut-level expectation, not merely internal parser terminology such as:

```text
unexpected token NEWLINE
```

when a clearer explanation exists.

---

## 32. Indentation Errors

Because indentation is semantic, indentation diagnostics must be precise.

Example:

```text
error[E0105]: unexpected indentation
  --> src/main.vut:6:5
   |
5  | print("A")
6  |     print("B")
   |     ^^^^ this line is indented too far
   |
   = expected indentation: 0 spaces
   = found indentation:    4 spaces
```

The diagnostic should distinguish:

- unexpected indent
- missing indent
- inconsistent indentation
- invalid dedent

---

## 33. Error Recovery

The compiler should continue after an error when safe recovery is possible.

The goal is to report multiple useful independent errors in one compile.

Example final summary:

```text
error: compilation failed due to 3 previous errors
```

---

## 34. Cascading Error Suppression

One syntax or semantic error must not generate a large number of meaningless secondary errors.

The compiler should mark invalid/error nodes and suppress diagnostics that are direct consequences of a previously reported root error.

Example:

A missing `:` should not cause every statement in the following block to be reported as unrelated undefined symbols.

---

## 35. Diagnostic Ordering

Diagnostics should normally be ordered by:

1. source/package
2. file
3. source position

When dependency errors block project compilation, the dependency identity should be clearly shown.

Internal compiler errors may be displayed separately because they represent compiler failures rather than source mistakes.

---

## 36. Dependency Diagnostics

Errors in installed packages must identify the package.

Example:

```text
error[E1003]: type mismatch
  --> package `math@1.2.0`:src/vector.vut:42:9
   |
42 | ...
```

Additional source metadata may be shown:

```text
= package source: github:duongonix/vpm/math
```

when useful.

---

## 37. VPM Diagnostics

VPM uses the same overall visual language but does not fabricate source frames for errors unrelated to `.vut` source.

Example:

```text
error: package `math` version `1.4.0` was not found

source:
  github:duongonix/vpm/math

requested:
  1.4.0

available:
  1.2.0
  1.3.0
```

---

## 38. VPM Dependency Collision

Example:

```text
error: dependency name `math` already exists

existing:
  source: github:duongonix/vpm/math
  version: 1.2.0

requested:
  source: github:nam/abc/math
  version: 2.0.0
```

VPM diagnostics need not use compiler `E####` codes unless a separate stable VPM code registry is later introduced.

---

## 39. Internal Compiler Errors

A compiler failure is different from a user source error.

Example:

```text
internal compiler error[E9001]: unexpected compiler state
```

The diagnostic should provide enough information for bug reporting without exposing irrelevant or unsafe internal data.

An internal compiler error must not be presented as if the user wrote invalid Vut syntax.

---

## 40. Machine-Readable Diagnostics

The compiler should support a machine-readable diagnostic mode for:

- IDEs
- editors
- language servers
- build systems
- CI

The exact format may be JSON or another explicitly documented structured format.

Structured diagnostics should contain at minimum:

```text
severity
code
message
file
primary span
related spans
notes
help
```

Human terminal output and machine output should originate from the same diagnostic model.

---

## 41. Diagnostic Data Model

Compiler stages should not directly print arbitrary error strings.

They should emit structured diagnostics.

Conceptually:

```text
Diagnostic
├── severity
├── code
├── title
├── primary_span
├── labels
├── related_spans
├── notes
└── help
```

Rendering belongs to the diagnostics layer.

Detailed implementation is defined in:

```text
specs/compiler/diagnostics.md
```

---

## 42. Source Span Preservation

All major compiler representations should preserve sufficient source-location information to generate precise diagnostics.

This includes where appropriate:

- tokens
- AST nodes
- declarations
- expressions
- types
- imports
- methods
- interface requirements

Lowering must not unnecessarily discard useful diagnostic origins.

---

## 43. Async Diagnostics

Async errors use the same structured model and codes, for example:

```text
error[E6011]: `await` is only valid inside an `async fn`
error[E6012]: cannot await a value of type `int`
error[E6013]: invalid async return type
error[E6014]: awaitable value must be awaited before use
```

Await diagnostics must point at the `await` expression or the offending operand,
show expected/found information where meaningful, and never panic the compiler.

Detailed examples belong to:

```text
specs/async/06-diagnostics.md
```

---

## 44. Diagnostic Principles

Vut diagnostics follow these principles:

1. Every source error identifies file, line and column.
2. Every source error displays relevant source code.
3. The primary error span is precise.
4. Source is syntax highlighted when supported.
5. Errors have stable `E####` codes.
6. Warnings use `W####`.
7. Expected/found information is shown when useful.
8. Related declarations are shown when relevant.
9. Help is only shown when reliable.
10. Multiple useful errors should be reported in one compile.
11. Cascading errors should be suppressed.
12. Human and machine diagnostics share one structured model.
13. VPM uses the same visual philosophy without fake source locations.
14. Compiler bugs are clearly separated from user mistakes.

This document is normative for Vut diagnostic behavior and presentation.
