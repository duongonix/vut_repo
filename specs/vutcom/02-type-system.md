# Vutcom — Type System and Ownership

> **Superseded.** The trailing `:` composition syntax is now owned by
> `specs/receiver/` (Receiver Functions). This document is kept for historical
> reference only and must not drive implementation. See
> `specs/receiver/00-overview.md`.

## 1. Domain Type

A declaration:

```vut
composition UI
```

introduces a nominal composition domain `UI`.

The primary associated type is:

```vut
vutcom[UI]
```

Different domains produce incompatible Vutcom types.

```vut
composition UI
composition Build
```

means:

```text
vutcom[UI] != vutcom[Build]
```

---

## 2. Type Checking

Given:

```vut
fn Home() -> vutcom[UI]:
  ...

fn Release() -> vutcom[Build]:
  ...
```

this is valid:

```vut
page: vutcom[UI] = Home()
```

This is invalid:

```vut
page: vutcom[UI] = Release()
```

No runtime domain check should be required.

---

## 3. Domain Parameters

`vutcom[D]` requires `D` to resolve to a composition domain.

The compiler should reject:

```vut
value: vutcom[int]
```

with a specific diagnostic.

Do not silently interpret arbitrary ordinary types as composition domains.

---

## 4. Function Return Checking

A function declared:

```vut
fn Home() -> vutcom[UI]:
  ...
```

must produce a valid UI composition on every normal completion path.

Composition-aware return analysis must integrate with normal Vut control-flow checking.

---

## 5. Child Domain Checking

Given:

```vut
fn Card(children: vutcom[UI]) -> vutcom[UI]:
  ...
```

the trailing block:

```vut
Card():
  ...
```

must be type checked as `vutcom[UI]`.

A Build composition inside the block must fail.

---

## 6. Leaf vs Container

A callable may accept a trailing composition block only if its signature supports child composition.

Example container:

```vut
fn Card(children: vutcom[UI]) -> vutcom[UI]:
  ...
```

Example leaf:

```vut
fn Text(value: str) -> vutcom[UI]:
  ...
```

The type checker must reject:

```vut
Text(value: "Hello"):
  Other()
```

This distinction must come from the callable signature, not from hardcoded component names.

---

## 7. Generic Functions

Vutcom must work with Vut generics.

Desired example:

```vut
fn ListView[T](
  items: list[T],
  render: fn(T) -> vutcom[UI]
) -> vutcom[UI]:
  Column():
    for item in items:
      render(item)
```

Usage:

```vut
ListView(
  items: users,
  render: user: > UserCard(user)
)
```

Vut's monomorphization strategy should remain applicable.

Do NOT introduce runtime generic dictionaries specifically for Vutcom.

---

## 8. Generic Domains

The implementation should avoid assumptions that prevent generic code from referring to composition domains where Vut's generic system can soundly support it.

However, do not invent higher-kinded types or a new generic system solely for Vutcom.

MVP should prioritize concrete domain parameters such as:

```vut
vutcom[UI]
vutcom[Build]
```

---

## 9. Cross-Domain Values as Ordinary Arguments

A domain may explicitly accept a composition from another domain.

Example conceptually:

```vut
fn Route(
  path: str,
  page: vutcom[UI]
) -> vutcom[Route]:
  ...
```

Then:

```vut
Route(
  path: "/",
  page: Home()
)
```

is valid.

This does NOT merge the UI and Route domains.

`vutcom[UI]` remains an ordinary typed argument owned by the Route API.

---

# Ownership

## 10. Ownership Direction

`vutcom[D]` must be opaque and non-Copy.

Canonical direction:

```text
vutcom[D]
=
opaque
+
non-Copy
+
move-oriented
```

Do NOT blindly classify it as trivially copyable.

A composition may capture:

- callbacks
- managed strings
- lists
- data
- resources
- other composition values
- future language-managed values

Duplicating a composition without ownership analysis could duplicate ownership incorrectly.

---

## 11. Linear Classification

Do NOT immediately hardcode:

```text
vutcom[D] = Linear
```

without integrating it with Vut's generic ownership model.

The implementation must evaluate Vutcom using the existing/planned ownership classification.

At minimum:

- Vutcom is not implicitly Copy.
- Moving a Vutcom value must obey move rules.
- Captured values must have correct ownership.
- Dropping a Vutcom value must correctly clean captured managed values/resources.
- No implicit duplicate owner may be created.

If the final ownership system classifies Vutcom as `Linear`, that classification must come from the generic ownership design rather than a Vutcom-specific hack.

---

## 12. Captures

Composition-producing functions may capture values.

Example:

```vut
fn UserCard(user: User) -> vutcom[UI]:
  Button(
    onclick: () => open(user.id)
  ):
    Text(value: user.name)
```

The compiler must determine what must survive with the resulting composition.

Do NOT automatically heap-box every local or every composition.

Use ordinary ownership and escape analysis.

Possible optimization strategies include:

- capture elision
- scalar replacement
- stack allocation
- direct specialization
- move into compact composition frame
- complete elimination after inlining

---

## 13. Composition Insertion

Given:

```vut
child: vutcom[UI] = Content()
```

and:

```vut
Column():
  child
```

the ownership effect must be defined consistently.

If insertion consumes/moves `child`, later use must follow normal move rules.

Do not silently clone composition values.

---

## 14. Consumer Ownership

A consumer such as:

```vut
ui.run(Home())
```

may consume the composition according to its normal function signature.

Vutcom itself must not introduce magical ownership exceptions.

The compiler should optimize:

```vut
ui.run(Home())
```

without changing source-level ownership semantics.

---

## 15. Deterministic Drop

If a materialized Vutcom value owns captures requiring destruction, they must be deterministically dropped.

This includes:

- normal completion
- early exit
- error paths
- moved values
- unused materialized composition
- consumer completion

No tracing GC is introduced for Vutcom.

---

## 16. Control Flow

Composition ownership must remain correct through:

- if
- else
- for
- match
- early return
- result propagation

Example:

```vut
fn Home(user: User?) -> vutcom[UI]:
  Column():
    if user:
      Profile(user)
    else:
      Login()
```

No branch may create invalid duplicate ownership.

---

## 17. Type Inference

Where the expected domain is known, the type checker may propagate the expected `vutcom[D]` type into nested composition.

Example:

```vut
fn Home() -> vutcom[UI]:
  Column():
    Text(value: "Hello")
```

The body context knows the expected domain is `UI`.

This must still be verified against the resolved signatures of `Column` and `Text`.

Do not infer domains from component names.

---

## 18. Diagnostics

Diagnostics must clearly explain Vutcom errors.

Examples:

### Domain mismatch

```text
expected `vutcom[UI]`
found `vutcom[Build]`
```

### Invalid domain

```text
`User` is not a composition domain
```

### Children not accepted

```text
`Text` does not accept child composition
```

### Child domain mismatch

```text
child composition requires `vutcom[UI]`
found `vutcom[Route]`
```

### Invalid composition call

```text
this callable cannot be used with a trailing composition block
```

Avoid leaking internal HIR/MIR concepts into user diagnostics.

---

## 19. No Runtime Type Tags Requirement

Domain type safety must primarily be compile-time.

Do NOT require runtime values such as:

```text
domain = "UI"
domain_id = hash("UI")
component_type = "Button"
```

for basic Vutcom semantics.

Runtime metadata may exist where genuinely required by a consumer, but that belongs to the consumer/library or optimized generic metadata system.

---

## 20. Invariants

The implementation must preserve:

```text
different composition domains are statically distinct

vutcom[D] is opaque

vutcom[D] is not implicitly Copy

cross-domain composition is explicit

captures obey normal Vut ownership

drop is deterministic where materialization requires drop

no domain-specific ownership hacks

no UI-specific type-system behavior
```