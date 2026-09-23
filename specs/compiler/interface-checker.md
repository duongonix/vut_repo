
# Vut Interface Checker

## 1. Purpose

The interface checker implements Vut's structural interface system.

No:

```text
impl
implements
```

declaration exists.

A type satisfies an interface automatically when its public method shape matches.

---

## 2. Responsibilities

Check:

```text
required methods
method names
parameter types
return types
visibility
interface composition
interface cycles
interface-to-interface compatibility
```

---

## 3. Structural Satisfaction

Example:

```vut
interface Animal:
  speak() -> str

data Dog:
  name: str

fn Dog.speak() -> str:
  "Woof"
```

`Dog` satisfies `Animal` automatically.

---

## 4. No Registration

Do not create a semantic requirement for:

```text
Dog implements Animal
```

No such syntax exists.

Compatibility is computed structurally.

---

## 5. Interface Shape

Represent normalized interface requirements.

Conceptually:

```text
InterfaceShape
├── method name
├── parameter types
└── return type
```

Use resolved `TypeId`.

Do not repeatedly compare textual type names.

---

## 6. Public Methods Only

Private:

```vut
fn Dog._speak() -> str:
```

does not satisfy:

```vut
interface Animal:
  speak() -> str
```

Likewise a private matching method cannot satisfy a public interface requirement.

---

## 7. Exact Signature Compatibility

Required:

```text
speak() -> str
```

Found:

```text
speak(int) -> str
```

is incompatible.

Report:

```text
E4103
```

---

## 8. Missing Method

When method is absent:

```text
E4102
```

with:

```text
missing method:
  speak() -> str
```

---

## 9. Interface Composition

Example:

```vut
interface ReadWriter: Reader, Writer:
  close()
```

Flatten/normalize parent requirements.

Duplicate identical method requirements should collapse.

---

## 10. Composition Conflict

If:

```text
ReaderA.read() -> str
ReaderB.read() -> bytes
```

and one interface inherits both, report:

```text
E4010
```

Do not arbitrarily select one signature.

---

## 11. Composition Cycle

Detect:

```text
A -> B -> C -> A
```

Report:

```text
E4011
```

Cycle detection should happen once while constructing the interface graph.

---

## 12. Interface-to-Interface

Interfaces may structurally satisfy other interfaces when their exposed behavior includes required shape.

No explicit inheritance declaration is required for pure structural compatibility.

Composition remains useful for declaring intentional grouped requirements.

---

## 13. Collections

For:

```text
list[Animal]
```

concrete values of different types may be accepted if each satisfies `Animal`.

Conversion/storage semantics are handled jointly with type/runtime lowering.

---

## 14. Call Sites

Example:

```vut
fn make_sound(animal: Animal):
  print(animal.speak())
```

Calling:

```vut
make_sound(dog)
```

triggers interface compatibility check.

---

## 15. Caching

Structural checks may occur frequently.

Cache results using:

```text
ConcreteTypeId
InterfaceId
```

Conceptually:

```text
(TypeId, InterfaceId) -> SatisfactionResult
```

Do not repeatedly traverse all methods for identical pairs.

---

## 16. Invalidations

Incremental compilation must invalidate cached satisfaction if:

```text
concrete public method set changes
interface requirements change
relevant type signatures change
visibility changes
```

---

## 17. Diagnostics

Missing:

```text
E4102
```

Mismatch:

```text
E4103
```

Private/non-public:

```text
E4104
```

Include relevant method/interface declaration spans.

---

## 18. Runtime Separation

Interface checker defines semantic compatibility.

It does not build machine vtables directly.

Runtime/codegen may later transform validated interface use into:

```text
data pointer
method table
```

or another efficient representation.

---

## 19. Tests

Test:

```text
single interface
multiple interfaces
missing method
wrong parameter type
wrong return type
private method
composition
duplicate compatible requirement
conflicting requirement
cycle
interface-to-interface
list[interface]
cache correctness
```

---

## 20. Rules

1. Satisfaction is automatic and structural.
2. No `impl`.
3. Interface matching uses normalized semantic signatures.
4. Private methods do not satisfy public interfaces.
5. Composition conflicts are errors.
6. Composition cycles are errors.
7. Cache repeated checks.
8. Preserve diagnostic source locations.
9. Semantic interface checking is separate from runtime dispatch.
10. Do not duplicate this logic elsewhere.
