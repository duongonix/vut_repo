# Vut Interfaces

## 1. Purpose

This document defines the interface system of Vut.

Vut interfaces provide behavioral abstraction without class inheritance.

The interface model is based on:

- structural compatibility
- automatic implementation
- method signatures
- multiple interfaces
- interface composition
- static checking
- predictable dispatch

Vut does not use:

```text
impl
implements
extends class
```

for interface implementation.

---

## 2. Interface Declaration

Interfaces use the `interface` keyword.

```vut
interface Animal:
  speak() -> str
```

An interface defines required behavior.

It does not create object state.

---

## 3. Interface Methods

Interface methods contain:

- method name
- parameters
- parameter types
- return type

Example:

```vut
interface Reader:
  read(size: int) -> bytes
```

Interface methods do not declare `self`.

Invalid:

```vut
interface Reader:
  read(self, size: int) -> bytes
```

`self` is implicit for instance behavior.

---

## 4. Interfaces Do Not Define Fields

Interfaces describe behavior only.

Do not define required fields such as:

```vut
interface User:
  name: str
  age: int
```

This is not valid interface behavior.

If state is required, use `data`.

```vut
data User:
  name: str
  age: int
```

Interfaces intentionally avoid structural field requirements.

---

## 5. Structural Implementation

A concrete type automatically satisfies an interface when it provides all required public methods with compatible signatures.

Example:

```vut
interface Animal:
  speak() -> str
```

Concrete type:

```vut
data Dog:
  name: str

fn Dog.speak() -> str:
  "Woof"
```

No additional declaration is required.

Do not write:

```text
impl Animal for Dog
```

or:

```text
Dog implements Animal
```

The compiler determines compatibility automatically.

---

## 6. Interface Compatibility Rules

A type `T` satisfies interface `I` when every required method of `I` is satisfied by `T`.

For each method:

1. method name must match
2. method must be public
3. parameter count must match
4. parameter order must match
5. parameter types must be compatible
6. return type must be compatible
7. the method must be available to the concrete type at the use site

Example:

```vut
interface Reader:
  read(size: int) -> bytes
```

Valid:

```vut
fn File.read(size: int) -> bytes:
  ...
```

Invalid:

```vut
fn File.read() -> bytes:
  ...
```

Invalid:

```vut
fn File.read(size: str) -> bytes:
  ...
```

Invalid:

```vut
fn File.read(size: int) -> str:
  ...
```

---

## 7. Automatic Interface Satisfaction

Interface satisfaction does not need to be declared in advance.

Example:

```vut
interface Printable:
  print()
```

Later:

```vut
data Document:
  text: str

fn Document.print():
  print(self.text)
```

`Document` automatically satisfies `Printable`.

This relationship may be discovered whenever a `Document` value is used where `Printable` is required.

---

## 8. Interface Function Parameters

Functions may accept interfaces.

```vut
fn make_sound(animal: Animal):
  print(animal.speak())
```

Concrete values satisfying the interface may be passed directly.

```vut
dog = Dog(name = "Milo")

make_sound(dog)
```

No explicit conversion is required.

---

## 9. Missing Methods

Given:

```vut
interface Animal:
  speak() -> str
```

and:

```vut
data Dog:
  name: str
```

this must fail:

```vut
make_sound(Dog(name = "Milo"))
```

if `Dog.speak()` does not exist.

The compiler should produce a diagnostic similar to:

```text
error[E4102]: `Dog` does not satisfy interface `Animal`
  --> src/main.vut:10:12
   |
10 | make_sound(Dog(name = "Milo"))
   |            ^^^^^^^^^^^^^^^^^^ `Dog` cannot be used as `Animal`
   |
   = missing method:
       speak() -> str
```

The diagnostic should also reference the interface declaration when useful.

---

## 10. Incompatible Method Signature

Example:

```vut
interface Animal:
  speak() -> str
```

Concrete method:

```vut
fn Dog.speak(volume: int) -> str:
  ...
```

This does not satisfy `Animal.speak()`.

Diagnostic concept:

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

## 11. Private Methods

Private methods do not satisfy public interface requirements.

Example:

```vut
interface Reader:
  read(size: int) -> bytes
```

This does not satisfy it:

```vut
fn File._read(size: int) -> bytes:
  ...
```

The names differ and `_read` is private.

If a required method itself is named with `_`, its visibility semantics follow the same module privacy rules as every other symbol.

---

## 12. Multiple Interfaces

A type may satisfy multiple interfaces automatically.

Example:

```vut
interface Reader:
  read(size: int) -> bytes

interface Writer:
  write(data: bytes) -> int
```

Concrete type:

```vut
data File:
  path: str

fn File.read(size: int) -> bytes:
  ...

fn File.write(data: bytes) -> int:
  ...
```

`File` satisfies both:

```text
Reader
Writer
```

No explicit declaration is necessary.

---

## 13. Interface Composition

Interfaces may compose other interfaces.

```vut
interface Reader:
  read(size: int) -> bytes

interface Writer:
  write(data: bytes) -> int

interface ReadWriter: Reader, Writer:
  close()
```

To satisfy `ReadWriter`, a type must provide:

```text
read(size: int) -> bytes
write(data: bytes) -> int
close()
```

---

## 14. Multiple Parent Interfaces

An interface may compose multiple interfaces.

```vut
interface A:
  a()

interface B:
  b()

interface C: A, B:
  c()
```

A concrete type satisfying `C` must satisfy all requirements from:

```text
A
B
C
```

---

## 15. Duplicate Requirements

If composed interfaces require identical method signatures, they represent a single compatible requirement.

Example:

```vut
interface A:
  close()

interface B:
  close()

interface C: A, B:
  flush()
```

A single compatible `close()` method may satisfy both inherited requirements.

---

## 16. Conflicting Requirements

If composed interfaces contain incompatible signatures with the same method name, the interface composition must fail.

Example:

```vut
interface A:
  read() -> str

interface B:
  read() -> bytes

interface C: A, B:
  ...
```

`C` contains an incompatible requirement.

The compiler must reject the interface declaration and identify both conflicting declarations.

---

## 17. Interface-to-Interface Structural Compatibility

Interfaces may themselves be structurally compatible.

Example:

```vut
interface A:
  run() -> int

interface B:
  run() -> int
```

A value satisfying `A` has the behavior required by `B`.

Where type-system rules permit interface conversion, the compiler may recognize this structural compatibility without requiring explicit inheritance between `A` and `B`.

Interface identity and interface structural compatibility must remain distinct concepts internally.

---

## 18. Interface Collections

Collections may use interface element types.

Example:

```vut
animals: list(Animal)
```

The list may contain:

```text
Dog
Cat
Bird
```

provided every concrete type satisfies `Animal`.

Example conceptually:

```vut
animals: list(Animal) = @(
  Dog(name = "Milo"),
  Cat(name = "Luna")
)
```

This is controlled heterogeneous storage based on shared static behavior.

It is not equivalent to:

```text
list(dyn)
```

---

## 19. Interface Return Values

Functions may return interface values.

Example:

```vut
fn create_animal() -> Animal:
  ...
```

The returned concrete value must satisfy `Animal`.

The exact runtime representation is an implementation concern.

The language-level behavior must remain independent from that representation.

---

## 20. Interface Fields Are Not Accessible

Because interfaces describe methods rather than fields:

```vut
fn print_name(animal: Animal):
  print(animal.name)
```

must fail unless `Animal` explicitly provides behavior exposing the name, for example:

```vut
interface Animal:
  name() -> str
```

Then:

```vut
fn print_name(animal: Animal):
  print(animal.name())
```

This prevents interface users from depending on concrete storage layouts.

---

## 21. Interface Method Calls

Given:

```vut
fn process(reader: Reader):
  data = reader.read(100)
```

the compiler knows `read` is available because it is required by `Reader`.

Other methods not present in the interface are unavailable through an interface-typed value unless the value is statically known as a more concrete type.

---

## 22. Interface Dispatch

Direct concrete method calls should use static/direct dispatch whenever possible.

Example:

```vut
dog.speak()
```

where `dog` is statically known as `Dog`.

Interface-typed calls may require dynamic dispatch.

Example:

```vut
fn make_sound(animal: Animal):
  animal.speak()
```

The expected runtime model may use an interface representation containing:

```text
data/reference pointer
method table
```

The exact representation belongs to compiler/runtime specifications.

The interface syntax must not expose vtables or dispatch implementation details to normal Vut source.

---

## 23. Interface Values

An interface value conceptually consists of:

```text
a concrete value
+
proof that its type satisfies the interface
```

The concrete underlying type remains distinct.

Interfaces do not transform concrete types into inheritance-based subclasses.

---

## 24. No Explicit Cast Required

If a concrete type satisfies an interface, it may be passed where that interface is required.

Example:

```vut
fn process(reader: Reader):
  ...

file = File(path = "data.txt")

process(file)
```

No syntax such as:

```text
file as Reader
```

should be required merely for standard interface satisfaction.

Explicit casts may still exist for other language purposes if specified separately.

---

## 25. Interface Satisfaction Is Compile-Time Checked

Where sufficient static information exists, interface compatibility must be verified during compilation.

Vut must not defer ordinary missing-method errors until runtime.

This is invalid at compile time:

```vut
fn process(reader: Reader):
  ...

value = SomeType()

process(value)
```

when `SomeType` does not satisfy `Reader`.

---

## 26. Generic Constraints

A generic type parameter may declare one or more interface bounds. Multiple
bounds are separated by `+`:

```vut
fn render(T: Printable + Named)(value: T) -> str:
  ...
```

A type argument satisfies a bound when the concrete type structurally satisfies
that interface. Satisfaction is validated at every concrete use of the generic
declaration, including generic function calls and generic type applications.

### 26.1 Constraint-Conditioned Method Access

Inside the body of a generic declaration, a value whose static type is a type
parameter `T` may use method-call syntax for any instance method declared by any
bound interface of `T`:

```vut
interface Encodable:
  to_json() -> Value

fn encode(T: Encodable)(value: T) -> str:
  stringify(value.to_json())
```

Rules:

1. **Eligible receiver.** Only a value whose static type is exactly a type
   parameter is eligible. Collections and composite types built from type
   parameters are not eligible in this revision; a value drawn from a container
   becomes eligible only once its static type is the type parameter itself.
2. **Eligible members.** Only instance methods declared by the parameter's
   declared bounds are accessible. Fields, associated/static members, and
   methods not present in any bound are not accessible.
3. **Signatures come from the bound.** Argument checking and the call result
   type use the requirement declared by the bound interface.
4. **Multiple bounds.** If any bound declares a method with the given name, the
   call is accepted. If two bounds declare the same name with incompatible
   signatures, the call is rejected as ambiguous.
5. **Static resolution.** Generic declarations are monomorphized. Each bound
   call is resolved to the implementing type's method of the same name for each
   concrete instantiation. No runtime dictionary, vtable, or indirect call is
   introduced for constraint-conditioned dispatch.
6. **Diagnostics.**
   - Method not declared by any bound: `E1018`, listing the bounds.
   - Ambiguous method across bounds: `E1019`.
   - Concrete type argument does not satisfy a bound: `E1017`, naming the
     missing, mismatched, or private method.
7. **No new syntax.** `T` does not become a callable or first-class type; only
   bound instance-method calls are permitted.

### 26.2 Scope of this revision

This revision specifies bound instance-method access on a type parameter.
Out of scope, to be specified separately:

- associated/static interface requirements and factory/conversion methods;
- implementations over type constructors such as `list(T)`, `map(K, V)`,
  `array(T, N)`, and `T?`;
- generic derive/metaprogramming;
- bounds with generic interface arguments (`T: Decoder(str)`);
- field/method access on a generic type application while its argument is still
  a type parameter (`b.get()` where `b: Box(T)`).

---

### 26.3 Associated Requirements and `Self`

An interface may declare **associated** (static) requirements by prefixing the
requirement with `static`. Static requirements have no implicit `self` and may
refer to the implementing type through `Self`:

```vut
interface Decodable:
  static from_json(value: Value) -> result(Self, Error)
```

The implementing type provides a matching associated function with `static fn`:

```vut
static fn User.from_json(value: Value) -> result(User, Error):
  ...
```

Rules:

1. `Self` is a contextual type name valid only inside an interface requirement.
   It denotes the type that satisfies the interface. Using `Self` elsewhere is an
   error.
2. A static requirement is satisfied only by a `static fn` with the same name and
   a compatible signature, after substituting `Self` with the implementing type.
   An instance method does not satisfy a static requirement, and a `static fn`
   does not satisfy an instance requirement.
3. A static member is called on a type or type parameter:
   - `Type.name(...)` for a concrete type;
   - `T.name(...)` where `T` is a type parameter bounded by an interface that
     declares a static requirement `name`.
4. A static member is not callable through a value (`value.name(...)`).
5. Static bound calls are resolved statically at monomorphization to the concrete
   associated function. No runtime dictionary or vtable is introduced.

---

## 27. Structural Compatibility and Extension Methods

If extension methods are introduced later, their effect on structural interface satisfaction must have explicit coherence rules.

The implementation must not automatically use arbitrary unrelated extension methods to satisfy interfaces until this behavior is specified.

---

## 28. Circular Interface Composition

Circular interface composition must be rejected.

Example:

```vut
interface A: B:
  a()

interface B: A:
  b()
```

The compiler should show the cycle clearly.

Example:

```text
A -> B -> A
```

---

## 29. Interface Visibility

Interfaces follow normal visibility rules.

Public:

```vut
interface Reader:
  read() -> bytes
```

Private:

```vut
interface _InternalReader:
  read() -> bytes
```

A private interface cannot be imported or referenced from outside its module.

---

## 29a. Async Methods and Interfaces

Interface requirements are synchronous in this phase.

An `async fn` method does not satisfy a synchronous interface requirement:

```vut
interface Loader:
  load() -> Data

data File:
  name: str

async fn File.load() -> Data:
  ...
```

`File` does not satisfy `Loader`, because the async method's call result is an
internal awaitable value, not `Data`.

Interfaces must not declare async requirements in this phase:

```vut
interface Loader:
  async load() -> Data   # invalid
```

Async interface requirements require a separate design decision and are deferred.

See:

```text
specs/async/02-type-system.md
```

---

## 30. Interface Principles

The Vut interface system follows these principles:

1. Interfaces describe behavior, not storage.
2. Interface methods do not declare `self`.
3. Implementation is structural and automatic.
4. There is no `impl` keyword.
5. There is no `implements` keyword.
6. Concrete types may satisfy multiple interfaces.
7. Interfaces may compose other interfaces.
8. Interface requirements must be type checked.
9. Private methods do not accidentally satisfy public requirements.
10. Interface collections may contain different compatible concrete types.
11. Concrete calls should remain statically dispatched where possible.
12. Interface dispatch implementation must not leak into source syntax.
13. Structural interface errors must provide precise diagnostics.
14. Interface cycles and signature conflicts are compile errors.
15. Interface requirements are synchronous; async methods do not satisfy them in this phase.

This document is normative for Vut's structural interface system.
