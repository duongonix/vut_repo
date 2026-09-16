# Vut Data Model

## 1. Purpose

This document defines Vut's structured data model.

The primary data mechanisms are:

- `data`
- lists
- enums
- composition

Vut does not use classes or tuples.

---

## 2. Named Data

Named structured types are declared using `data`.

```vut
data User:
  name: str
  age: int
```

A `data` declaration introduces a named type.

Fields are declared using:

```text
name: Type
```

No commas are required between fields.

---

## 3. Data Construction

A `data` value is constructed using the type name.

```vut
user = User(
  name = "Ha",
  age = 20
)
```

The resulting type is inferred as:

```text
User
```

An explicit annotation is unnecessary:

```vut
user: User = User(
  name = "Ha",
  age = 20
)
```

but may be written when useful.

---

## 4. Required Fields

Fields without defaults are required.

```vut
data User:
  name: str
  age: int
```

Valid:

```vut
user = User(
  name = "Ha",
  age = 20
)
```

Invalid:

```vut
user = User(
  name = "Ha"
)
```

The compiler must report that `age` is missing.

Vut must not silently leave fields uninitialized.

---

## 5. Default Fields

Fields may define default values.

```vut
data Counter:
  value: int = 0
```

Construction may omit fields with defaults.

```vut
counter = Counter()
```

Override:

```vut
counter = Counter(
  value = 100
)
```

Default expressions must satisfy the declared field type.

Invalid:

```vut
data Counter:
  value: int = "zero"
```

---

## 6. Field Access

Fields use dot access.

```vut
user.name
user.age
```

Fields may be modified when the value and field are mutable under the memory model.

```vut
user.age = 21
```

Exact value/reference/move semantics are defined in:

```text
specs/08-memory-model.md
```

---

## 7. Private Fields

Fields beginning with `_` are private to the defining module.

```vut
data User:
  name: str
  _password: str
```

Code inside the defining module may access:

```vut
user._password
```

Code in another module must not access it.

The compiler must produce a privacy diagnostic when this rule is violated.

---

## 8. No Classes

Vut does not define a `class` construct.

Do not introduce:

```text
class User
```

State belongs in `data`.

Behavior is attached through methods.

```vut
data Counter:
  value: int = 0

fn Counter.increment():
  self.value = self.value + 1
```

---

## 9. No Inheritance

Named data types do not inherit from other data types.

Do not introduce:

```text
extends
inherits
super
```

for data inheritance.

Reuse uses composition.

---

## 10. Composition

A `data` type may contain another `data` type.

```vut
data Position:
  x: float
  y: float

data Player:
  name: str
  position: Position
```

Construction:

```vut
player = Player(
  name = "Ha",
  position = Position(
    x = 10.0,
    y = 20.0
  )
)
```

Access:

```vut
player.position.x
player.position.y
```

Composition is the primary mechanism for building complex object models.

---

## 11. Methods and Data

Methods may be attached to named data types.

```vut
fn Player.move(x: float, y: float):
  self.position.x = x
  self.position.y = y
```

Usage:

```vut
player.move(20.0, 30.0)
```

Method declarations and `self` behavior are defined in:

```text
specs/04-functions-methods.md
```

---

## 12. Named Data and No Tuples

Vut does not provide tuples.

Do not treat:

```text
(10, 20)
```

as a tuple literal.

```vut
data Point:
  x: int
  y: int
```

---

## 15. Lists

List literals use `@()`.

```vut
numbers = @(1, 2, 3)
```

List types use:

```text
list(T)
```

Example:

```vut
numbers: list(int) = @(1, 2, 3)
```

List elements must be compatible with the list element type.

---

## 16. Lists of Data

Lists may contain named data values.

```vut
data User:
  name: str

users = @(
  User(name = "Ha"),
  User(name = "Nam")
)
```

The inferred type is conceptually:

```text
list(User)
```

---

## 17. Lists of Interfaces

A list may use an interface element type.

Example:

```vut
interface Animal:
  speak() -> str
```

Different concrete values may coexist in:

```text
list(Animal)
```

provided every value satisfies `Animal`.

This is the preferred mechanism for heterogeneous collections based on shared behavior.

---

## 18. Dynamic Lists

When arbitrary unrelated values are intentionally required:

```vut
values: list(dyn) = @(
  10,
  "hello",
  true
)
```

`dyn` must be explicit.

The compiler must not infer `list(dyn)` merely because incompatible elements were supplied.

---

## 19. Empty Lists

An empty list may require explicit type context.

```vut
users: list(User) = @()
```

Without contextual information:

```vut
items = @()
```

the compiler must not arbitrarily invent an element type.

---

## 20. Enums

Enums define a finite set of variants and each variant may carry named payload
fields.

```vut
enum Shape:
  point
  circle(radius: float)
  rect(width: float, height: float)
```

A variant with no payload is written as a bare name. A variant with a payload
lists `name: Type` fields, matching `data` field syntax.

Enum values are statically typed.

A payloadless variant is referenced through its enum:

```vut
shape = Shape.point
```

A payload variant is constructed with named arguments:

```vut
shape = Shape.circle(radius = 2.0)
```

`Shape.circle(radius = 2.0)` constructs the `circle` variant with `radius`
bound. The compiler validates field count, field names, and field types.

Qualification (`Shape.variant`) is required for construction and remains
consistent across parser, formatter, and module specifications.

Enum payload types may not contain the enum by value; a recursive shape must use
an indirect container such as `list(T)`. Direct recursion is a compile error.

---

## 21. Enum Matching

Enums integrate with `match`.

```vut
text = match shape:
  point: "point"
  circle(radius = r): "circle $(r)"
  rect(width = w, height = h): "rect $(w)x$(h)"
```

The compiler verifies match exhaustiveness for finite enums. Every variant must
be covered unless a wildcard `_` or binding pattern handles the remainder.

Detailed control-flow and pattern semantics are defined in:

```text
specs/05-control-flow.md
```

---

## 22. Payload Enums

Enum variants may carry values, as defined above.

Payload fields are named and typed:

```vut
enum Event:
  quit
  key(code: int)
  resize(width: int, height: int)
```

Patterns bind payload fields by name:

```vut
match event:
  quit: ...
  key(code = c): ...
  resize(width = w, height = h): ...
```

A single-field variant may use the positional shorthand `key(c)`.

Generic enums such as `Option(T)` are not available until generic declaration
syntax is finalized; concrete payload types are fully supported.

Each variant value is a tagged union: a tag plus the payload of the active
variant. Only the active payload owns managed data and is destroyed.

---

## 23. Result and Option-Like Data

Error and optional-value abstractions should eventually be represented using strongly typed data/enum mechanisms rather than exceptions.

Examples conceptually include:

```text
result(T, E)
optional(T)
```

Optional shorthand:

```text
T?
```

Exact standard-library representation is defined separately.

---

## 24. Map

Vut is intended to provide a typed map collection:

```text
map(K, V)
```

Example type:

```vut
scores: map(str, int)
```

The final map literal and construction API are not yet locked.

A map represents a collection of key/value pairs and has different semantics.

---

## 25. Data Equality

Equality uses:

```text
==
!=
```

The exact equality behavior for:

- named data
- lists
- maps
- enums

must be defined by their type semantics and standard-library contracts.

Reference identity must not be silently substituted for value equality unless explicitly specified.

---

## 26. Mutability

Normal variables and data values are mutable unless constrained by another language rule.

Example:

```vut
user.age = 21
```

ALL-CAPS bindings are constants.

```vut
DEFAULT_USER = User(
  name = "Ha",
  age = 20
)
```

The exact depth of const protection and copy/reference behavior is defined by the memory model.

Do not guess deep/shallow const semantics in the parser or type checker before that specification is complete.

---

## 27. Data Safety

Vut must not expose uninitialized safe-language fields.

The following must always be true for normal safe Vut code:

- required fields are initialized
- default fields receive valid values
- field assignments satisfy their declared types
- private fields respect module visibility
- invalid field names are compile errors

Example:

```vut
user.unknown = 10
```

must fail when `unknown` is not a field of `User`.

---

## 28. Unknown Fields During Construction

Invalid:

```vut
user = User(
  name = "Ha",
  age = 20,
  unknown = true
)
```

If `User` does not define `unknown`, the compiler must report an error.

The diagnostic should identify:

- the invalid field
- the `User` declaration
- available fields when useful

---

## 29. Duplicate Fields During Construction

Invalid:

```vut
user = User(
  name = "Ha",
  name = "Nam",
  age = 20
)
```

The compiler must reject duplicate field initialization.

The diagnostic should reference both occurrences where possible.

---

## 30. Data-Model Principles

The Vut data model follows these principles:

1. `data` defines named structured state.
2. Methods define behavior separately.
3. Composition replaces inheritance.
4. Vut has no anonymous keyed records or tuples.
7. Lists are homogeneous by default.
8. `dyn` is required for arbitrary heterogeneous values.
9. Fields are always initialized in safe code.
10. Named data types retain nominal identity.
11. Interfaces provide structural behavior abstraction.
12. Memory semantics are defined independently from surface data syntax.

This document is normative for Vut structured-data syntax and semantics unless a more specialized specification explicitly defines a narrower behavior.
