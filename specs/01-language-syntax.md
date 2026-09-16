# Vut Language Syntax

## 1. Purpose

This document defines the core lexical and syntactic conventions of the Vut programming language.

Vut is a statically typed, compiled, indentation-based programming language designed around:

- concise syntax
- strong static typing
- type inference
- predictable behavior
- minimal boilerplate
- readable compiler diagnostics
- high native performance

Source files use the `.vut` extension.

Example:

```vut
name = "Vut"
version = 1

if version > 0:
  print(name)
```

---

## 2. Source Files

Every Vut source file uses the `.vut` extension.

Examples:

```text
main.vut
user.vut
math.vut
http_client.vut
```

Every `.vut` file is automatically a module.

A module declaration is not required.

Do not introduce syntax such as:

```text
module math
```

Module and import semantics are defined separately in:

```text
specs/07-modules-imports.md
```

---

## 3. Blocks and Indentation

Vut uses indentation to define blocks.

Vut does not use braces for blocks.

Valid:

```vut
if active:
  print("active")
```

Invalid:

```vut
if active {
  print("active")
}
```

A block begins after `:`.

Example:

```vut
if age >= 18:
  print("adult")
else:
  print("minor")
```

Nested blocks increase indentation:

```vut
if active:
  if admin:
    print("admin")
```

The canonical formatter uses two spaces for each indentation level.

```vut
if active:
  print("A")

  if admin:
    print("B")
```

The compiler must detect inconsistent or unexpected indentation and produce a precise diagnostic.

---

## 4. Statement Termination

Vut uses newlines to terminate statements.

Semicolons are not required.

```vut
name = "Ha"
age = 20
print(name)
```

Vut code must not require:

```vut
name = "Ha";
age = 20;
```

---

## 5. Comments

### 5.1 Single-line comments

Single-line comments begin with `#`.

```vut
# This is a comment

name = "Vut" # inline comment
```

### 5.2 Multi-line comments

Multi-line comments use `##` delimiters.

```vut
##
This is a
multi-line comment.
##
```

### 5.3 Documentation comments

Documentation comments use:

```vut
### This is documentation
```

Documentation comments are intended for declarations and documentation tooling.

Detailed documentation semantics may be expanded by the documentation specification.

---

## 6. Identifiers

Identifiers may contain:

- letters
- digits
- underscore

An identifier must not begin with a digit.

Valid:

```vut
name
user_name
user2
_http_client
MAX_SIZE
```

Invalid:

```text
2user
```

Identifiers are case-sensitive.

Therefore:

```text
user
User
USER
```

are distinct identifiers.

Case also carries language meaning in some contexts:

- identifiers beginning with `_` are private
- ALL-CAPS variable bindings are constants

---

## 7. Variables

Variables are declared through assignment.

```vut
name = "Ha"
age = 20
active = true
```

No `let` or `var` keyword is required.

The compiler infers the type from the initial value.

Example:

```vut
age = 20
```

is inferred as an integer type.

Once inferred, the variable's type cannot change.

```vut
age = 20
age = 21
```

is valid.

The following is invalid:

```vut
age = 20
age = "twenty"
```

because the type of `age` was already inferred as an integer.

---

## 8. Explicit Type Annotations

Types may be written explicitly.

```vut
name: str = "Ha"
age: int = 20
active: bool = true
```

Type annotations are optional when the compiler can infer the type.

The following two declarations should represent the same logical type:

```vut
age = 20
```

```vut
age: int = 20
```

Detailed type inference rules are defined in:

```text
specs/02-type-system.md
```

---

## 9. Dynamic Values

Vut is statically typed by default.

Dynamic behavior must be explicitly requested using `dyn`.

```vut
value: dyn = 10

value = "hello"
value = true
```

Without `dyn`, changing the type of a variable is a compile error.

`dyn` must never be inferred automatically merely because incompatible values are assigned.

---

## 10. Constants

ALL-CAPS bindings are constants.

```vut
MAX_SIZE = 100
PI = 3.14159
APP_NAME = "Vut"
```

Vut does not require a `const` keyword.

After initialization, a constant cannot be reassigned.

Invalid:

```vut
MAX_SIZE = 100
MAX_SIZE = 200
```

The compiler must report an error at the reassignment and reference the original declaration when possible.

---

## 11. Privacy Naming Convention

Identifiers beginning with `_` are private to their module.

```vut
_token = "abc"

fn _parse():
  ...
```

Identifiers without `_` are public by default.

```vut
name = "Ha"

fn parse():
  ...
```

Vut does not use:

```text
pub
public
private
export
```

Privacy applies consistently to:

- variables
- functions
- methods
- fields
- data types
- interfaces
- enums
- module symbols

Detailed module visibility semantics are defined in:

```text
specs/07-modules-imports.md
```

---

## 12. Boolean Literals

Boolean literals are:

```vut
true
false
```

They are lowercase keywords.

---

## 13. Null Literal

The null literal is:

```vut
null
```

`null` is primarily used with optional types and other explicitly supported nullable contexts.

Example:

```vut
nickname: str? = null
```

A normal non-optional value must not silently accept `null`.

---

## 14. String Literals

Strings use double quotes.

```vut
name = "Vut"
message = "Hello world"
```

The canonical Vut syntax uses double-quoted strings.

String escaping and Unicode behavior must be formally defined by the lexer specification.

String literals are template strings. `$identifier` interpolates a simple
identifier and `$(expression)` embeds a normal Vut expression. `\$` writes a
literal dollar. Interpolations are parsed and type checked at compile time;
they do not use runtime evaluation or `dyn` conversion.

---

## 15. Numeric Literals

Integer:

```vut
age = 20
count = 100
```

Floating point:

```vut
price = 19.99
pi = 3.14159
```

Detailed numeric typing and representation are defined in:

```text
specs/02-type-system.md
```

---

## 16. List Literals

Vut does not use square brackets for list literals.

A list literal uses `@()`.

```vut
numbers = @(1, 2, 3)
```

Explicit type:

```vut
numbers: list(int) = @(1, 2, 3)
```

Lists are homogeneous by default.

Valid:

```vut
numbers = @(1, 2, 3)
names = @("Ha", "Nam", "Lan")
```

Invalid:

```vut
values = @(1, "hello", true)
```

Mixed values require an explicitly dynamic element type.

```vut
values: list(dyn) = @(1, "hello", true)
```

---

## 17. Structured Data and No Tuple Type

Vut does not have tuples.

Do not introduce tuple semantics for:

```text
(a, b)
```

When structured data is useful, declare and construct named `data`. Vut has no
anonymous record literal; `$()` is reserved exclusively for template-string
expression interpolation.

```vut
data Point:
  x: int
  y: int

point = Point(x = 10, y = 20)
```

---

## 19. Arithmetic Operators

Vut supports the standard arithmetic operators:

```text
+
-
*
/
%
```

Example:

```vut
total = a + b
remaining = total - used
area = width * height
ratio = a / b
rest = value % 2
```

Operator type rules belong to the type-system specification.

---

## 20. Comparison Operators

Vut uses:

```text
==
!=
>
<
>=
<=
```

Examples:

```vut
name == "Ha"
age != 10
score > 100
score >= 50
```

Vut does not use `is` as an equality operator.

`is` is not part of the Vut language.

---

## 21. Logical Operators

Vut uses word-based logical operators:

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

Vut does not use:

```text
&&
||
!
```

for boolean logic.

---

## 22. Member Access

Member access uses `.`.

```vut
user.name
user.age
```

Method calls also use `.`.

```vut
user.login()
counter.increment()
```

Module access uses the same syntax.

```vut
math.add()
```

The semantic meaning is determined by the type or module associated with the expression.

---

## 23. Function Calls

Function calls use parentheses.

```vut
print("hello")
add(10, 20)
```

Named construction also uses call syntax:

```vut
user = User(
  name = "Ha",
  age = 20
)
```

Function and method semantics are defined in:

```text
specs/04-functions-methods.md
```

---

## 24. Type Application Syntax

Parameterized types use parentheses.

```vut
list(int)
map(str, int)
result(User, Error)
ptr(int)
```

Vut does not use angle-bracket generic syntax such as:

```text
List<int>
Map<String, Int>
```

Generic semantics are defined by the type-system specification.

---

## 25. Keywords and Reserved Syntax

The language currently reserves or uses concepts including:

```text
fn
data
interface
enum
type

async
await

if
elif
else
match

for
in
break
continue

return

import
as
at

and
or
not

true
false
null

dyn
```

Additional keywords must not be introduced casually.

Vut intentionally aims to keep its keyword set small.

---

## 26. Async Functions and Await

An asynchronous function prefixes `fn` with `async`:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

The declared return type is the logical result type.

`await` is a prefix expression and is only valid inside an `async fn`:

```vut
value = await operation()
```

Allowed forms include:

```vut
return await operation()
process(await operation())
value = await operation()?
```

`await operation()?` parses as `(await operation())?`; `await` and `?` are
independent mechanisms.

Vut does not use:

```text
operation().await
async { ... }
await(...)         # as a distinct call form
```

Detailed behavior belongs to:

```text
specs/async/
```

`async main` is allowed and is driven by the runtime.

---

## 27. General Syntax Principles

All future Vut syntax must follow these principles:

1. Prefer concise syntax.
2. Prefer readability over punctuation-heavy syntax.
3. Avoid unnecessary keywords.
4. Avoid braces for blocks.
5. Avoid semicolons.
6. Preserve static analyzability.
7. Avoid ambiguous grammar.
8. Prefer compile-time errors over implicit runtime behavior.
9. Keep syntax consistent across language features.
10. New syntax must integrate with precise compiler diagnostics.

This document is normative for the core surface syntax of Vut.
