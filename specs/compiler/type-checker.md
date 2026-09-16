

# Vut Type Checker

Every template expression is checked as a normal expression. Its resolved static
type selects formatting; template interpolation must neither infer `dyn` nor
weaken ordinary type errors.

## 1. Purpose

The type checker enforces Vut's static type system.

It is responsible for:

```text
type inference
type compatibility
assignments
function arguments
return types
operators
collections
optionals
data fields
control-flow expression types
```

It must follow:

```text
specs/02-type-system.md
```

---

## 2. Static Typing

Every ordinary Vut expression has a statically known type before code generation.

`dyn` is explicitly dynamic.

Do not silently introduce `dyn` to resolve type conflicts.

---

## 3. Inference

Example:

```vut
age = 20
```

compiler infers a numeric type according to literal inference rules.

After binding type is established:

```vut
age = "hello"
```

must fail.

---

## 4. Variable Type Stability

A variable's static type does not change after initial inference/declaration.

Conceptually:

```text
Binding
├── SymbolId
└── TypeId
```

---

## 5. Explicit Annotations

Example:

```vut
age: int = 20
```

The initializer must be compatible with the annotation.

Mismatch:

```text
E1003
```

---

## 6. Type Representation

Use a centralized semantic type system.

Conceptually:

```text
TypeData
├── Bool
├── Integer(...)
├── Float(...)
├── Str
├── Bytes
├── Dyn
├── Null
├── Optional(TypeId)
├── List(TypeId)
├── Map(...)
├── Data(DataId)
├── Enum(EnumId)
├── Interface(InterfaceId)
├── Function(...)
├── Future(TypeId)   # internal awaitable, not source-nameable
└── Error
```

Exact internals may evolve.

`Future(TypeId)` is compiler-internal. It is produced by calling an `async fn`
and consumed by `await`. It must not be exposed as source syntax.

---

## 7. Type Interner

Equivalent types should preferably canonicalize to the same `TypeId`.

Example:

```text
list(int)
```

should not allocate a completely new type object on every occurrence.

Use a type interner/arena where beneficial.

---

## 8. Error Type

Use an internal error/unknown type to suppress cascading errors.

If:

```text
unknown_symbol
```

already produced an error, dependent arithmetic should not generate meaningless additional type errors.

---

## 9. Literal Types

Numeric literals may initially use literal-specific inference representations.

Example:

```text
IntegerLiteral(20)
```

can later resolve to:

```text
int
i32
u8
...
```

based on context.

Do not prematurely force every literal into one fixed-width type if the language rules do not require it.

---

## 10. Lists

For:

```vut
@(1, 2, 3)
```

infer:

```text
list(int)
```

or equivalent context-selected element type.

For:

```vut
@(1, "hello")
```

report:

```text
E1005
```

unless expected type is explicitly:

```text
list(dyn)
```

and conversion semantics permit it.

---

## 11. Empty Lists

Example:

```vut
items = @()
```

without type context should report:

```text
E1004
```

if element type cannot be inferred.

With context:

```vut
items: list(int) = @()
```

is valid.

---

## 12. Optional Types

For:

```text
T?
```

`null` may be valid.

For non-optional `T`:

```text
null
```

must fail.

Exact narrowing/unwrapping semantics remain deferred where not finalized.

Do not invent unsafe implicit unwrap.

---

## 14. `dyn`

`dyn` accepts values of arbitrary Vut types according to dynamic boxing rules.

Costs and runtime representation must remain localized to actual `dyn` usage.

Static values should not be converted to dynamic representation unnecessarily.

---

## 15. Operators

Operator typing must be centralized.

Example:

```text
+
```

may support allowed numeric/string cases according to language/std semantics.

Invalid:

```vut
"hello" - 10
```

→

```text
E1009
```

Do not let codegen decide type legality.

---

## 16. Boolean Conditions

These require:

```text
bool
```

exactly:

```vut
if condition:
for condition:
```

No truthiness.

Example:

```vut
if 10:
```

is a type/control-flow error.

---

## 17. Comparisons

Validate operands for:

```text
==
!=
>
<
>=
<=
```

according to compatible-type rules.

Do not implement `is`.

---

## 18. Function Calls

Check:

```text
argument count
argument types
named argument validity
```

Use declaration spans as related diagnostic locations.

---

## 19. Methods

Method call:

```vut
user.login()
```

uses receiver type to resolve available methods.

Implicit self is included semantically but not in source-level argument count.

---

## 20. Returns

For:

```vut
fn get() -> int:
  "hello"
```

report return mismatch.

If return type is inferred, collect compatible return/final-expression constraints.

---

## 20a. Async and Await

Calling an `async fn` whose logical result is `T` produces `Future(T)`.

```text
async fn load() -> int   ->   load() : Future(int)
```

`await` requires an operand of `Future(T)` and yields `T`:

```text
await e : T     where e : Future(T)
```

Rules:

```text
await is valid only inside an async function/method body
await inside a non-async lambda is invalid
await on a non-awaitable value is an error (E6012)
using a Future value as an ordinary value is an error (E6014)
no implicit await
no implicit conversion to/from Future
```

`await operation()?` is typed as `(await operation())?`:

```text
operation()          : Future(result(T, E))
await operation()    : result(T, E)
await operation()?   : T
```

Async function bodies are checked against their logical return type. Returning
an awaitable value without `await` is an error (E6013 or the general return
mismatch when the types are unrelated).

Async functions are not ordinary function values in this phase:
`async fn` cannot satisfy an expected `fn(...) -> ...`.

---

## 21. `if` Expression Types

Branches used as values must satisfy type rules.

Do not silently promote incompatible branches to `dyn`.

Example:

```vut
value = if cond:
  1
else:
  "hello"
```

must fail unless a surrounding explicitly supported dynamic context makes the conversion legal.

---

## 22. Match Types

All value-producing match arms must satisfy compatible result typing.

Enum exhaustiveness integrates with control-flow analysis.

Pattern checking validates:

```text
pattern kind matches the scrutinee type
variant patterns name a known variant
every payload field is bound or `_`
`or` alternatives bind identical names and compatible types
guards are bool
bindings are not duplicated within a pattern
```

Exhaustiveness and reachability use a coverage analysis over enum variants,
boolean literals, and list shapes; a wildcard or binding makes the remainder
exhaustive. Unreachable arms are reported as `W5004`.

---

## 23. Data Construction

Validate:

```text
required fields
field types
defaults
unknown fields
duplicate fields
```

Data-specific diagnostics may live in data semantic validation while using the shared type checker.

Avoid duplicate type logic.

---

## 24. Type Aliases

Aliases resolve to aliased types.

They do not create new nominal identity under current rules.

---

## 25. Interfaces

Assignment/call compatibility involving interfaces delegates structural satisfaction to:

```text
InterfaceChecker
```

Do not duplicate structural matching logic inside arbitrary type-check functions.

---

## 26. Incremental Architecture

Type checking should support per-module/per-item caching later.

Avoid one global mutable pass where every source change invalidates everything unnecessarily.

---

## 27. Performance

Avoid:

```text
recursive string type comparisons
recreating identical types
linear search through all types
repeated interface shape checking
```

Use:

```text
TypeId
interning
caches
symbol IDs
```

---

## 28. Diagnostics

Type diagnostics must contain exact spans.

Example:

```text
error[E1003]: type mismatch
  --> src/user.vut:12:9
```

Include expected/found where meaningful.

---

## 29. Tests

Test categories:

```text
inference
annotations
fixed binding types
numeric literals
lists
list dyn
empty lists
optional
null
operators
calls
returns
if expression
match
data construction
methods
interfaces
aliases
error suppression
```

---

## 30. Rules

1. Type checker is authoritative for type compatibility.
2. Inference never means dynamic typing.
3. Variable types remain fixed.
4. Do not silently introduce `dyn`.
5. Use canonical TypeIds.
6. Numeric literal inference remains context-aware.
7. Boolean conditions require bool.
8. Interface compatibility delegates to interface checker.
9. Type errors preserve exact source locations.
10. Codegen receives already type-checked programs.





