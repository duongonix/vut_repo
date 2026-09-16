# Vut Type System

## 1. Purpose

This document defines the core type-system rules of Vut.

Vut uses:

- static typing
- compile-time type checking
- type inference
- fixed variable types
- explicit dynamic typing through `dyn`
- homogeneous typed collections
- optional types
- structural interface compatibility

The type system should reject invalid programs before native code generation whenever sufficient information is available.

---

## 2. Static Typing

Vut is statically typed.

Every normal value has a type known to the compiler.

Example:

```vut
age = 20
name = "Ha"
active = true
```

The compiler determines the types during compilation.

Type annotations are not required when the type can be inferred.

---

## 3. Type Inference

The compiler infers a variable's type from its initial assignment.

```vut
age = 20
name = "Ha"
active = true
```

Conceptually:

```text
age    → int
name   → str
active → bool
```

Explicit forms are also valid:

```vut
age: int = 20
name: str = "Ha"
active: bool = true
```

Type inference must never mean dynamic typing.

---

## 4. Fixed Variable Types

Once a variable receives a type, later assignments must remain compatible with that type.

Valid:

```vut
count = 10
count = 20
count = 30
```

Invalid:

```vut
count = 10
count = "ten"
```

The compiler must reject the second assignment.

Example diagnostic concept:

```text
error[E1003]: type mismatch
  --> src/main.vut:2:9
   |
2  | count = "ten"
   |         ^^^^^ expected `int`, found `str`
```

---

## 5. Explicit Types

A variable may declare its type explicitly.

```vut
count: int = 10
name: str = "Vut"
active: bool = true
```

The assigned value must be compatible with the declared type.

Invalid:

```vut
count: int = "ten"
```

The compiler must not silently convert incompatible values merely to satisfy an annotation.

---

## 6. Core Primitive Types

The core type system includes the following primitive or fundamental types.

### Boolean

```text
bool
```

Values:

```vut
true
false
```

### Signed integers

```text
i8
i16
i32
i64
```

### Unsigned integers

```text
u8
u16
u32
u64
```

### General integer

```text
int
```

`int` is the default general-purpose integer type unless contextual inference requires another numeric type.

Its exact ABI representation is defined by the compiler/runtime specification.

### Floating point

```text
f32
f64
```

### General floating point

```text
float
```

`float` is the default general-purpose floating-point type.

Its exact representation is defined separately.

### String

```text
str
```

### Bytes

```text
bytes
```

`bytes` is a core managed type representing a contiguous buffer of binary
data. Its element type is `u8` with values in `0..=255`.

`bytes` is distinct from both `str` and `list(u8)`:

```text
str       = text / Unicode string
bytes     = raw binary buffer
list(u8)  = generic collection of u8
```

`bytes` is not an alias of `list(u8)`. It has its own semantic type and
metadata so compiler and runtime can optimize it and integrate it with I/O
and FFI.

Element access follows the list method model with `u8` elements:

```vut
data.at(index) -> u8
data.set(index, value: u8)
data.first() -> u8
data.last() -> u8
```

The following must be rejected:

```vut
data.set(0, "A")
```

Conversions are explicit:

```vut
values = data.to_list()          # bytes -> list(u8)
data = bytes.from_list(values)   # list(u8) -> bytes
blob = text.to_bytes()           # str -> bytes (UTF-8)
text = data.to_str()             # bytes -> result(str, Utf8Error)
```

`bytes` and `str` are never implicitly interchangeable. UTF-8 validation is
performed by `to_str`; it never reinterprets or silently replaces invalid
input.

### Dynamic

```text
dyn
```

### Null literal

```text
null
```

`null` is not a general replacement for typed values.

It is primarily valid in explicitly nullable/optional contexts.

---

## 7. Numeric Literal Inference

An integer literal without stronger context is inferred as `int`.

```vut
value = 10
```

Conceptually:

```text
value: int
```

A floating-point literal without stronger context is inferred as `float`.

```vut
value = 10.5
```

Conceptually:

```text
value: float
```

Explicit numeric types may provide contextual typing.

```vut
small: i8 = 10
large: u64 = 100
ratio: f32 = 0.5
```

The compiler must reject compile-time numeric literals that cannot fit the requested type.

Example:

```vut
value: u8 = 300
```

must fail because the literal is outside the valid `u8` range.

---

## 8. Dynamic Type

Dynamic behavior must be explicit.

```vut
value: dyn = 10

value = "hello"
value = true
```

This is valid because `value` is explicitly `dyn`.

Without `dyn`:

```vut
value = 10
value = "hello"
```

must fail.

The compiler must never automatically convert a statically typed variable to `dyn` because incompatible assignments occur.

---

## 9. Optional Types

Optional types use:

```text
T?
```

Example:

```vut
nickname: str? = null
```

A non-null value of the underlying type is also valid:

```vut
nickname: str? = "Ha"
```

A normal non-optional value cannot accept `null`.

Invalid:

```vut
name: str = null
```

The exact narrowing/unwrapping syntax will be defined before optional values are fully implemented.

Unsafe access to an optional value must not silently succeed.

---

## 10. List Types

Lists use:

```text
list(T)
```

Example:

```vut
numbers: list(int) = @(1, 2, 3)
```

The compiler can infer the type:

```vut
numbers = @(1, 2, 3)
```

as:

```text
list(int)
```

Lists are homogeneous.

Invalid:

```vut
values = @(1, "hello", true)
```

The compiler must reject heterogeneous elements unless the element type explicitly permits them.

Dynamic list:

```vut
values: list(dyn) = @(1, "hello", true)
```

---

## 11. Empty Collections

An empty collection does not necessarily contain enough information to infer its element type.

Therefore an explicit type may be required.

Example:

```vut
items: list(int) = @()
```

The compiler must not arbitrarily guess an element type for an unconstrained empty list.

---

## 12. Named Data Types

A `data` declaration introduces a named type.

```vut
data User:
  name: str
  age: int
```

Construction:

```vut
user = User(
  name = "Ha",
  age = 20
)
```

The compiler infers:

```text
user: User
```

Named data types are nominal identities.

Two independently declared `data` types are not automatically the same type merely because their fields are identical.

Structural compatibility in Vut is specifically used for interfaces.

---

## 13. No Anonymous Record or Tuple Type

Vut does not define a tuple type.

Do not introduce:

```text
tuple(T, U)
```

as a core Vut type unless the language specification is intentionally revised.

Structured values should use:

- `data`
- dedicated collection types

---

## 15. Function Types

Function parameters are statically typed.

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Arguments must satisfy parameter types.

Invalid:

```vut
add("1", 2)
```

Return expressions must satisfy the declared or inferred return type.

Detailed function typing is defined in:

```text
specs/04-functions-methods.md
```

---

## 16. Type Inference for Functions

When no explicit return type is provided, the function return type is `void`.

Example:

```vut
fn log_sum(a: int, b: int):
  out("$(a + b)")
```

is a `void` function. A function returning a value must declare its return type explicitly.

The default is statically checked; returning a value from an
omitted-return-type function is an error.

---

## 17. Interface Types

Interfaces describe required behavior.

```vut
interface Animal:
  speak() -> str
```

A concrete type satisfies the interface automatically when it provides all required public methods with compatible signatures.

```vut
data Dog:
  name: str

fn Dog.speak() -> str:
  "Woof"
```

`Dog` therefore satisfies `Animal`.

No explicit:

```text
impl Animal for Dog
implements Animal
```

is required.

---

## 18. Structural Interface Compatibility

Interface satisfaction is structural.

For a type `T` to satisfy interface `I`:

1. every required interface method must exist
2. required methods must be publicly accessible
3. method names must match
4. parameter counts must match
5. parameter types must be compatible
6. return types must be compatible

Example:

```vut
interface Reader:
  read(size: int) -> bytes
```

This method satisfies it:

```vut
fn File.read(size: int) -> bytes:
  ...
```

This does not:

```vut
fn File.read() -> bytes:
  ...
```

The compiler must report the missing or incompatible signature precisely.

---

## 19. Private Methods and Interfaces

Private methods do not satisfy public interface requirements across module boundaries.

Example:

```vut
fn File._read(size: int) -> bytes:
  ...
```

does not satisfy:

```vut
interface Reader:
  read(size: int) -> bytes
```

Names are different and `_read` is private.

---

## 20. Multiple Interfaces

A concrete type may satisfy any number of interfaces automatically.

No declaration is required.

Compatibility is evaluated where the type is used.

---

## 21. Interface Composition

Interfaces may extend/combine other interfaces.

```vut
interface Reader:
  read(size: int) -> bytes

interface Writer:
  write(data: bytes) -> int

interface ReadWriter: Reader, Writer:
  close()
```

A type satisfying `ReadWriter` must provide all requirements inherited from `Reader`, `Writer`, and `ReadWriter`.

---

## 22. Interface Collections

Collections may use interface element types.

Conceptually:

```vut
animals: list(Animal)
```

may contain different concrete types provided every element satisfies `Animal`.

This enables controlled heterogeneous collections without requiring `dyn`.

---

## 23. Type Aliases

Vut supports type aliases.

```vut
type UserId = u64
type Names = list(str)
```

A type alias provides another name for a type.

It does not automatically create a distinct nominal type.

Therefore:

```vut
type UserId = u64
```

is an alias of `u64`, not a new incompatible numeric type.

---

## 24. Parameterized Types

Parameterized type syntax uses parentheses.

```vut
list(int)
map(str, int)
result(User, Error)
ptr(int)
```

Vut does not use:

```text
List<int>
Map<String, Int>
```

Generic declaration and constraint semantics are defined in `06-interfaces.md`
§26 (`+` separates multiple bounds).

---

## 25. Type Conversion

Vut should prefer explicit conversion over broad implicit coercion.

The intended style is:

```vut
value.to_int()
value.to_float()
value.to_str()
```

The standard library determines which conversions exist.

The compiler must not silently convert unrelated types merely to make an expression compile.

---

## 26. Equality

Equality uses:

```text
==
!=
```

Examples:

```vut
a == b
a != b
```

`is` is not an equality operator and is not part of Vut.

Detailed equality semantics for complex values are defined by their respective type specifications.

---

## 27. Boolean Conditions

Conditions used by control-flow constructs must be `bool`.

Valid:

```vut
if active:
  ...
```

Invalid:

```vut
if 10:
  ...
```

Vut does not use arbitrary truthiness as a replacement for boolean typing.

The same rule applies to conditional loops:

```vut
for active:
  ...
```

The expression after `for` must be `bool`.

---

## 28. Constants and Types

ALL-CAPS bindings are constants.

```vut
MAX_SIZE = 100
```

The compiler still infers a normal static type:

```text
MAX_SIZE: int
```

Constness and type are separate properties.

The constant cannot be reassigned after initialization.

---

## 29. Compile-Time Type Errors

Type errors must be compile errors whenever statically detectable.

Examples include:

- incompatible assignment
- invalid function argument
- invalid return type
- heterogeneous list
- invalid `null`
- invalid interface implementation
- non-boolean condition
- impossible numeric literal
- invalid field assignment

The compiler must produce precise source spans.

---

## 30. Type-System Principles

The Vut type system follows these rules:

1. Static typing is the default.
2. Type inference reduces boilerplate.
3. Inference must not weaken type safety.
4. Variable types remain fixed.
5. Dynamic behavior requires explicit `dyn`.
6. Collections are typed and homogeneous by default.
7. `null` requires a compatible nullable context.
8. Interface compatibility is structural.
9. Named `data` types retain nominal identity.
10. Implicit conversions should remain limited.
11. Invalid type behavior should be rejected as early as possible.
12. Compiler diagnostics must explain both expected and actual types.

---

## 31. Fixed Array Types

Fixed arrays use `array(T, N)`, where `N` is a compile-time integer greater
than zero and participates in type identity. `array(int, 4)` and
`array(int, 8)` are therefore different types.

`array(e1, ..., eN)` always constructs `array(T, N)`, while `@(e1, ..., eN)`
always constructs `list(T)`. Both forms are homogeneous unless their expected
element type is explicitly `dyn`; neither form is contextually converted into
the other.

## 32. Async Function and Await Types

An asynchronous function declares a logical result type:

```vut
async fn load() -> int:
  ...
```

The user-facing signature is `async fn() -> int`.

Calling an async function produces a compiler-internal awaitable value:

```text
load() : future(int)
```

`future(T)` is not writable in Vut source. It is not a standard-library type and
has no public operations other than `await`.

`await` requires an awaitable operand and yields the logical result:

```text
e : future(T)
---------------
await e : T
```

`await` is valid only inside an `async fn`. Applying `await` to a non-awaitable
value is a type error, and no implicit await or implicit conversion is allowed.

`await` and `?` are independent. For:

```vut
data = await read_async()?
```

the typing is:

```text
read_async()          : future(result(T, E))
await read_async()    : result(T, E)
await read_async()?   : T
```

Async functions are not ordinary function values in this phase; they cannot be
passed where `fn(...) -> ...` is expected.

Detailed behavior belongs to:

```text
specs/async/02-type-system.md
```

---

## 33. Deferred Type-System Details

The following require dedicated specification before implementation is considered complete:

- generic function declaration syntax
- generic constraints
- exact `int` ABI
- exact `float` ABI
- optional narrowing and unwrapping
- `result(T, E)` semantics
- `?` propagation
- pointer types
- ownership/reference semantics
- map typing
- capturing closure typing

These details must not be guessed by the implementation.

They must be specified before their corresponding compiler features are finalized.
