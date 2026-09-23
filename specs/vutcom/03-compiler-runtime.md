# Vutcom — Compiler and Runtime Architecture

> **Superseded.** The trailing `:` composition syntax is now owned by
> `specs/receiver/` (Receiver Functions). This document is kept for historical
> reference only and must not drive implementation. See
> `specs/receiver/00-overview.md`.

## 1. Goal

Vutcom must be implemented as a generic compiler capability.

Canonical architecture:

```text
Vut source
    ↓
Parser / AST
    ↓
HIR + type checking
    ↓
typed composition representation
    ↓
Vutcom lowering
    ↓
MIR
    ↓
ownership / optimization
    ↓
codegen
    ↓
native program
```

The implementation must remain domain-agnostic.

---

## 2. Parser

The parser must support:

```vut
composition UI
```

and trailing composition blocks:

```vut
Column():
  Text(value: "Hello")
```

The parser must NOT create UI-specific AST nodes.

Possible generic AST concepts:

```text
CompositionDomainDecl
CallExpr
TrailingCompositionBlock
```

Normal calls without a trailing block must continue using ordinary call parsing.

---

## 3. Symbol Resolution

Composition domain declarations must enter the appropriate type/symbol namespace.

The resolver must distinguish:

- composition domains
- ordinary types
- functions
- data constructors
- modules

Call-like syntax must be resolved using normal symbol resolution.

Do NOT infer semantics based on capitalization or component names.

---

## 4. HIR

HIR may introduce generic composition concepts such as:

```text
ComposeCall
ComposeBlock
ComposeBranch
ComposeLoop
ComposeValue
```

Exact names are implementation details.

HIR MUST NOT contain:

```text
UiNode
ButtonNode
TextNode
RouteNode
BuildNode
WidgetNode
```

HIR should retain enough information for:

- domain type
- callable
- arguments
- child block
- control-flow composition
- source locations
- ownership analysis
- later optimization

---

## 5. Type Checking

Type checking must verify:

- valid `composition D`
- valid `vutcom[D]`
- function return domain
- nested composition domain
- trailing block compatibility
- children parameter compatibility
- leaf/container distinction
- cross-domain restrictions
- callback/function argument types
- generic specialization compatibility

Domain errors must be caught before codegen.

---

## 6. Vutcom Lowering Pass

Composition lowering should be implemented as a dedicated generic compiler pass/subsystem rather than scattering domain-specific behavior throughout codegen.

Conceptual flow:

```text
typed HIR
   ↓
composition analysis
   ↓
Vutcom lowering
   ↓
ordinary/specialized MIR
   ↓
normal optimization
```

The pass may lower composition into:

- specialized calls
- compact frames
- static metadata
- branch regions
- loop regions
- consumer-compatible internal operations

The exact representation is internal.

---

## 7. Primitive Operations

Domain libraries need a generic way to define primitive composition operations.

The compiler may recognize a primitive operation as:

```text
primitive operation of composition domain D
```

but must not know its semantic meaning.

For example, the compiler may know:

```text
Primitive<UI>
```

but not:

```text
Button
Text
Column
```

Likewise it may know:

```text
Primitive<Build>
```

but not:

```text
Compile
Package
Test
```

If a new declaration or ABI is required for primitives, design it generically.

Do not add one-off compiler intrinsics for example domains.

---

## 8. No Mandatory Node Tree

The compiler must not lower every composition into:

```text
allocate Node
allocate children list
append child
append child
return root
```

unless a specific consumer explicitly requires such representation.

Source nesting is not a promise of runtime tree allocation.

---

## 9. Static and Dynamic Regions

The compiler should be capable of distinguishing static and dynamic composition regions.

Example:

```vut
fn Home(name: str) -> vutcom[UI]:
  Column():
    Text(value: "Vut")
    Text(value: name)
```

Conceptually:

```text
Column             static structure
Text("Vut")        static value
Text(name)         dynamic value
```

This information may enable optimization.

It must remain generic and not UI-specific.

---

## 10. Control-Flow Lowering

Example:

```vut
Column():
  if logged_in:
    Profile()
  else:
    Login()
```

may become a generic composition branch region.

Likewise:

```vut
Column():
  for user in users:
    UserCard(user: user)
```

may become a generic repeated region.

Do NOT lower these to UI concepts such as:

```text
ConditionalWidget
ForEachWidget
```

Reuse Vut CFG and control-flow infrastructure wherever possible.

---

## 11. Intermediate Value Elimination

Source:

```vut
ui.run(Home())
```

is semantically equivalent to producing a `vutcom[UI]` and passing it to `run`.

However, the optimizer should be free to eliminate the intermediate value.

Conceptually:

```text
Home
 ↓
specialized composition
 ↓
consumer
```

instead of:

```text
Home
 ↓
heap Vutcom object
 ↓
consumer
```

when semantics permit.

---

## 12. Inlining

Composition-producing functions should participate in ordinary compiler inlining.

Example:

```vut
fn Label(value: str) -> vutcom[UI]:
  Text(value: value)

fn Home() -> vutcom[UI]:
  Column():
    Label(value: "Hello")
```

The optimizer may inline `Label`.

Do not introduce an abstraction barrier that prevents normal optimization.

---

## 13. Generics and Monomorphization

Vutcom should work with Vut's generic monomorphization.

Example:

```vut
fn ListView[T](
  items: list[T],
  render: fn(T) -> vutcom[UI]
) -> vutcom[UI]:
  ...
```

Specialization should occur using normal generic infrastructure.

No Vutcom-specific runtime generic dispatch should be introduced unless generic Vut itself requires it.

---

## 14. Capture Optimization

Composition values may capture data.

The compiler should integrate with:

- escape analysis
- move analysis
- copy elision
- scalar replacement
- stack allocation
- deterministic drop

Do NOT automatically allocate one closure/frame per composition call.

---

## 15. Runtime Boundary

Vutcom core should require as little mandatory runtime machinery as practical.

There must NOT be a universal runtime assumption such as:

```text
every composition is a heap Node tree
```

A consumer may provide its own runtime.

Examples:

```text
UI library
→ renderer + state system

Build library
→ scheduler + dependency graph

Route library
→ route table

Workflow library
→ workflow executor
```

These are not part of Vutcom core.

---

## 16. Composition Consumer

A consumer remains an ordinary function/API.

The compiler must not special-case:

```vut
ui.run(...)
build.execute(...)
server.serve(...)
```

If future optimization recognizes consumer patterns, it must use generic mechanisms rather than hardcoded library names.

---

## 17. Composition Identity

The compiler architecture should leave room for generic composition identity.

Possible future internal representation:

```text
CompositionRegionId
CallSiteIdentity
LoopItemIdentity
```

This can support domain libraries requiring stable identity.

Do not implement UI-specific state keys as part of MVP.

---

## 18. Ownership Integration

Vutcom must integrate with Vut's generic ownership model.

The compiler must correctly handle:

- move of composition values
- captures
- nested compositions
- branch-specific composition
- loop composition
- deterministic destruction
- consumer ownership

Do NOT introduce:

```text
VutcomRetain
VutcomGC
UIReferenceCount
```

unless equivalent behavior comes from generic Vut ownership mechanisms.

---

## 19. Runtime Allocation Goals

Target behavior:

```text
simple static composition
→ ideally zero unnecessary heap allocations

nested composition
→ inline/specialize where profitable

temporary vutcom value
→ eliminate where possible

captured composition
→ allocate only when lifetime/escape requires it
```

These are optimization goals, not excuses to violate semantics.

---

## 20. No String-Based Dispatch

Do NOT implement primitive dispatch using patterns such as:

```text
"Button"
"Text"
"Column"
```

or:

```text
map<string, handler>
```

for core dispatch.

Primitive resolution should be statically typed whenever possible.

---

## 21. Example Architecture

Source:

```vut
fn Home(users: list[User]) -> vutcom[UI]:
  Column():
    Text(value: "Users")

    for user in users:
      UserCard(user: user)
```

Conceptual compiler pipeline:

```text
Home
 ↓
typed vutcom[UI]
 ↓
ComposeCall(Column)
 ├── ComposeCall(Text)
 └── ComposeLoop(users)
       └── ComposeCall(UserCard)
 ↓
generic composition lowering
 ↓
specialized MIR
 ↓
inline / specialize / eliminate temporaries
 ↓
native code
```

The actual MIR representation may differ.

---

## 22. Testing Requirements

Compiler tests must include:

### Parsing

- composition declaration
- simple composition call
- nested trailing blocks
- multiple children
- ordinary call without children

### Type system

- valid same-domain composition
- invalid cross-domain composition
- invalid domain argument
- leaf receiving children
- explicit cross-domain typed argument
- generic composition

### Control flow

- if
- if/else
- nested if
- for
- nested for
- match where supported
- composition value insertion

### Ownership

- move composition
- use after move
- captured str
- captured list
- captured data
- captured resource where supported
- branch cleanup
- loop cleanup
- deterministic drop

### Optimization

- nested component inlining
- temporary composition elimination
- no mandatory heap Node allocation
- static composition optimization

### Regression

All existing compiler/runtime tests must remain green.

---

## 23. Forbidden Implementations

Do NOT:

- hardcode UI into compiler
- hardcode Build into compiler
- hardcode Route into compiler
- create a mandatory Virtual DOM
- allocate a Node for every composition call
- use runtime strings for domain type checking
- bypass ownership rules
- implicitly clone `vutcom[D]`
- introduce a tracing GC for Vutcom
- create special callback semantics only for UI
- create Vutcom-specific `if` or `for`
- create HTTP-specific or framework-specific composition behavior
- place all Vutcom logic into one compiler file

The implementation must be modular.

Create dedicated modules/files where necessary.

---

## 24. Success Criterion

The architecture is successful when the same compiler mechanism can support:

```text
vutcom[UI]
vutcom[Build]
vutcom[Route]
vutcom[Workflow]
```

without adding domain-specific compiler logic for each new domain.