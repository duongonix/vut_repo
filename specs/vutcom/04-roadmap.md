# Vutcom — Implementation Roadmap

> **Superseded.** The trailing `:` composition syntax is now owned by
> `specs/receiver/` (Receiver Functions). This document is kept for historical
> reference only and must not drive implementation. See
> `specs/receiver/00-overview.md`.

## 1. Goal

Implement Vutcom completely as a generic typed declarative composition system.

The implementation must proceed incrementally.

Do NOT attempt to implement the entire subsystem in one uncontrolled change.

After every phase:

1. format code
2. run focused tests
3. run compiler tests
4. run workspace regression tests
5. fix regressions before continuing
6. update this roadmap with completion status

Do not mark a phase complete while known correctness failures introduced by that phase remain.

---

# Phase 0 — Repository Investigation

Before changing code:

- inspect parser
- inspect AST
- inspect HIR
- inspect type representation
- inspect symbol resolution
- inspect function call resolution
- inspect MIR builder
- inspect ownership metadata
- inspect monomorphization
- inspect codegen
- inspect module/import system

Identify where Vutcom integrates naturally.

Produce an implementation plan based on actual repository architecture.

Do not invent parallel compiler infrastructure if existing generic infrastructure can be extended.

---

# Phase 1 — Composition Domain Syntax

Implement:

```vut
composition UI
```

Requirements:

- parser support
- AST representation
- source spans
- symbol registration
- duplicate declaration diagnostics
- nominal domain identity

Tests:

```vut
composition UI
composition Build
```

must create distinct domains.

---

# Phase 2 — `vutcom(D)` Type

Implement generic composition type syntax:

```vut
vutcom(UI)
```

Requirements:

- type representation
- type parsing/resolution
- equality
- diagnostics
- formatting/debug representation
- integration with type metadata

Reject:

```vut
vutcom(int)
```

unless the argument resolves to a composition domain.

---

# Phase 3 — Function Return Support

Support:

```vut
fn Home() -> vutcom(UI):
  ...
```

Requirements:

- function signature
- return checking
- type inference integration
- module boundaries
- imported composition domains

Do not introduce a new component declaration keyword.

---

# Phase 4 — Composition Call and Trailing Block Parsing

Implement:

```vut
Column():
  Text(value = "Hello")
```

Parser must support:

- call + trailing `:`
- nested blocks
- multiple children
- source spans
- normal calls unchanged

Keep syntax generic.

---

# Phase 5 — Callable Resolution

Resolve composition calls using ordinary symbol resolution.

Support:

- function calls
- data constructors where valid
- imported symbols
- aliases where normal Vut supports them

Do not resolve by capitalization or runtime strings.

Detect ambiguous/conflicting symbols using normal namespace rules.

---

# Phase 6 — Child Composition

Implement child block typing.

Canonical model:

```vut
fn Card(
  children: vutcom(UI)
) -> vutcom(UI):
  ...
```

Usage:

```vut
Card():
  Text(value = "Hello")
```

The trailing block conceptually supplies `children`.

No public `children(D)` type.

---

# Phase 7 — Leaf / Container Validation

Given:

```vut
fn Text(value: str) -> vutcom(UI):
  ...
```

reject:

```vut
Text(value = "Hello"):
  Other()
```

Add clear diagnostics.

Do not hardcode leaf names.

Derive capability from callable signature.

---

# Phase 8 — Multiple Children

Support:

```vut
Column():
  A()
  B()
  C()
```

Multiple children must compose without requiring the user to construct a list manually.

Design the internal representation to avoid mandatory list/heap allocation.

---

# Phase 9 — Composition Value Insertion

Support:

```vut
fn Card(children: vutcom(UI)) -> vutcom(UI):
  Panel():
    children
```

Requirements:

- type checking
- ownership
- move semantics
- nested insertion
- diagnostics

---

# Phase 10 — Domain Type Safety

Implement complete domain checking.

Valid:

```vut
ui.run(Home())
```

Invalid:

```vut
ui.run(BuildProject())
```

Expected style:

```text
expected `vutcom(UI)`
found `vutcom(Build)`
```

No runtime string domain checks.

---

# Phase 11 — Explicit Cross-Domain Arguments

Support APIs such as:

```vut
fn Route(
  path: str,
  page: vutcom(UI)
) -> vutcom(Route):
  ...
```

Allow:

```vut
Route(
  path = "/",
  page = Home()
)
```

while still rejecting implicit insertion of UI composition into Route composition.

---

# Phase 12 — Ordinary Logic

Verify composition-producing functions support normal Vut code:

- variables
- expressions
- helper functions
- callbacks
- ordinary calls
- result handling where valid

Example:

```vut
fn Home(user: User) -> vutcom(UI):
  title = user.name

  fn click():
    out(title)

  Button(onclick = click):
    Text(value = title)
```

Do not create a restricted Vutcom-only function body language.

---

# Phase 13 — If / Else Composition

Support:

```vut
Column():
  if logged_in:
    Profile()
  else:
    Login()
```

Requirements:

- CFG integration
- domain checking
- branch typing
- ownership correctness
- cleanup

Do not implement an `If` component.

---

# Phase 14 — For Composition

Support:

```vut
Column():
  for user in users:
    UserCard(user = user)
```

Requirements:

- normal Vut iteration semantics
- composition region lowering
- ownership correctness
- nested loops
- nested composition

Do not implement a `ForEach` component.

---

# Phase 15 — Match Composition

Where Vut match semantics permit, support composition-producing match branches.

Requirements:

- compatible domains
- exhaustiveness rules remain normal Vut rules
- ownership correctness
- branch cleanup

Do not create Vutcom-specific pattern matching.

---

# Phase 16 — Primitive Composition Mechanism

Design and implement a generic way for libraries to provide primitive composition operations.

Requirements:

- domain-aware
- statically resolved
- no UI-specific semantics
- no string dispatch
- usable by external libraries
- suitable for optimization
- clear compiler/library boundary

The compiler may know:

```text
primitive operation of domain D
```

but must not know:

```text
Button
Text
Route
Compile
```

Document the final primitive mechanism before depending on it broadly.

---

# Phase 17 — Dedicated HIR Representation

Introduce generic typed composition representation where needed.

Potential concepts:

```text
ComposeCall
ComposeBlock
ComposeBranch
ComposeLoop
ComposeValue
```

Names may differ based on repository architecture.

Never introduce domain-specific nodes.

---

# Phase 18 — Vutcom Lowering Pass

Implement a dedicated generic lowering stage.

Target architecture:

```text
HIR
 ↓
typed composition
 ↓
Vutcom lowering
 ↓
MIR
 ↓
optimization
 ↓
codegen
```

Avoid scattering Vutcom-specific transformations across unrelated codegen paths.

---

# Phase 19 — Ownership Integration

Integrate `vutcom(D)` with Vut's ownership system.

Requirements:

- non-Copy
- move correctness
- use-after-move diagnostics
- deterministic cleanup
- nested composition ownership
- child insertion ownership
- consumer ownership

Do not implement implicit cloning.

Do not add Vutcom-specific reference counting unless generic ownership semantics genuinely require it.

---

# Phase 20 — Capture Analysis

Support composition values capturing:

- str
- list
- data
- callbacks
- managed values
- resources where generic ownership allows

Integrate with:

- escape analysis
- move analysis
- drop analysis
- copy elision

Avoid universal heap boxing.

---

# Phase 21 — Generic Composition

Support generic functions producing Vutcom.

Example:

```vut
fn ListView(T)(
  items: list(T),
  render: fn(T) -> vutcom(UI)
) -> vutcom(UI):
  Column():
    for item in items:
      render(item)
```

Use normal Vut monomorphization.

Do not add runtime dictionaries specifically for Vutcom.

---

# Phase 22 — Consumer Integration

Verify Vutcom can be passed to ordinary functions:

```vut
fn run(root: vutcom(UI)):
  ...
```

and:

```vut
ui.run(Home())
```

The compiler must not special-case `run`.

Also verify multiple consumers are possible:

```vut
build.execute(plan)
build.inspect(plan)
```

---

# Phase 23 — Intermediate Composition Elimination

Optimize:

```vut
ui.run(Home())
```

so an unnecessary materialized intermediate `vutcom(UI)` can be eliminated when safe.

Target:

```text
source composition
↓
specialized lowering
↓
consumer
```

instead of mandatory:

```text
heap composition object
↓
consumer
```

Preserve observable ownership semantics.

---

# Phase 24 — Inlining and Specialization

Ensure composition-producing functions participate in:

- inlining
- generic specialization
- constant propagation
- dead code elimination
- copy elision

Example:

```vut
fn Label(value: str) -> vutcom(UI):
  Text(value = value)
```

should not create an unavoidable runtime abstraction layer.

---

# Phase 25 — Static/Dynamic Region Optimization

Analyze composition regions for static and dynamic values.

Example:

```vut
Column():
  Text(value = "Vut")
  Text(value = name)
```

The compiler may identify:

```text
static structure
static constant
dynamic value
```

Use this information only where safe and profitable.

Keep the mechanism domain-agnostic.

---

# Phase 26 — Allocation Optimization

Target:

- no mandatory Node allocation
- no mandatory child list allocation
- stack allocation where possible
- scalar replacement
- escape-based materialization
- compact captures
- temporary elimination

Add tests/inspection to ensure simple composition does not regress into unnecessary heap-heavy behavior.

---

# Phase 27 — Diagnostics

Add high-quality diagnostics for:

- invalid composition domain
- domain mismatch
- invalid child composition
- leaf receiving children
- invalid trailing block
- ambiguous call resolution
- ownership violation
- use after move
- invalid cross-domain insertion

Diagnostics must use source-level terminology.

Do not expose internal HIR/MIR implementation details.

---

# Phase 28 — Module and Package Integration

Verify composition domains and Vutcom APIs work across:

- files
- modules
- imports
- packages
- aliases
- stdlib/external libraries

A third-party package must be able to define its own domain without compiler modification.

Example:

```vut
composition Scene
```

must not require adding `Scene` to compiler source.

---

# Phase 29 — Generic Domain Proof

Create at least three independent test/example domains.

Required:

```text
UI
Build
Route
```

They do not need to be production frameworks.

They exist to prove compiler genericity.

The same Vutcom infrastructure must compile all three without domain-specific compiler branches.

---

# Phase 30 — Performance Validation

Create representative benchmarks.

Measure:

- compilation cost
- runtime overhead
- allocations
- nested composition
- loops
- generic composition
- captures
- direct consumer invocation

Compare optimized composition against equivalent ordinary Vut code where meaningful.

Investigate unnecessary overhead.

Do not sacrifice correctness for benchmark results.

---

# Phase 31 — Composition Identity Foundation

Add only the generic foundation necessary for stable composition identity if required by future consumers.

Potential internal concepts:

```text
call-site identity
composition region identity
loop-item identity
```

Do NOT add:

```text
remember
state
recompose
widget key
```

to Vutcom core.

UI-specific state semantics belong to UI libraries.

If identity is not required for the completed generic MVP, document the extension point and defer implementation.

---

# Phase 32 — Full Regression and Hardening

Run:

- parser tests
- type checker tests
- ownership tests
- MIR tests
- codegen tests
- module tests
- package tests
- optimization tests
- workspace tests

Test combinations:

- deeply nested composition
- empty child block where legal
- many children
- nested if/else
- nested loops
- match
- generics
- callbacks
- managed captures
- move-only captures
- explicit cross-domain values
- imported domains
- multiple consumers

Fix all Vutcom-introduced correctness regressions.

---

# Phase 33 — Documentation Sync

After implementation, update canonical specs to match actual finalized behavior.

Do not silently change locked semantics because implementation was easier another way.

If implementation reveals a fundamental conflict with existing Vut architecture:

1. stop
2. document the conflict
3. explain alternatives
4. request a design decision

Do not silently introduce a workaround that changes Vutcom semantics.

---

# Completion Criteria

Vutcom is considered complete only when all of the following work:

```vut
composition UI
composition Build
composition Route
```

and:

```vut
fn Home() -> vutcom(UI):
  Column():
    Text(value = "Users")

    if logged_in:
      Profile()

    for user in users:
      UserCard(user = user)
```

and generic composition works:

```vut
fn ListView(T)(
  items: list(T),
  render: fn(T) -> vutcom(UI)
) -> vutcom(UI):
  Column():
    for item in items:
      render(item)
```

and cross-domain safety works:

```text
vutcom(UI) != vutcom(Build) != vutcom(Route)
```

and external libraries can define new domains without compiler modification.

---

# Final Architecture Invariant

The completed system must preserve:

```text
                         Vut Compiler
                              │
                    Generic Vutcom System
                              │
             typed declarative composition
                              │
                         vutcom(D)
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
          ▼                   ▼                   ▼
     vutcom(UI)         vutcom(Build)       vutcom(Route)
          │                   │                   │
          ▼                   ▼                   ▼
      UI Library          Build Library       Route Library
```

The compiler understands:

```text
composition
domain
vutcom(D)
children
composition control flow
ownership
lowering
optimization
```

The compiler does NOT understand:

```text
Button
Text
Column
Widget
DOM
Renderer
Route semantics
Build semantics
Workflow semantics
```

---

# Forbidden Shortcuts

The agent MUST NOT complete Vutcom by:

- implementing only UI use cases
- hardcoding example domain names
- creating a universal heap Node tree
- creating a mandatory Virtual DOM
- using strings as type identity
- bypassing ownership
- cloning compositions implicitly
- adding a tracing GC
- putting all implementation into one large file
- duplicating existing compiler infrastructure
- breaking existing Vut syntax
- changing locked Vutcom semantics without explicit approval

The agent may create new files/folders/modules where appropriate.

Prefer clear modular compiler architecture over concentrating unrelated logic in existing large files.

---

# Definition of Done

Vutcom is 100% complete when:

1. All locked language semantics in `specs/vutcom/` are implemented.
2. UI, Build and Route proof domains work through the same generic mechanism.
3. Nested children work.
4. Leaf/container validation works.
5. if/else works.
6. for works.
7. match works where normal Vut semantics permit.
8. callbacks work.
9. generic composition works.
10. explicit cross-domain arguments work.
11. implicit cross-domain composition fails.
12. ownership and deterministic cleanup are correct.
13. managed/resource captures are correct where supported by generic Vut ownership.
14. external packages can define domains.
15. consumers are ordinary library functions.
16. no mandatory Node/Virtual DOM representation exists.
17. unnecessary intermediate composition values can be optimized away.
18. no domain-specific compiler logic exists.
19. diagnostics are clear.
20. all existing regression tests remain green.
21. relevant benchmarks demonstrate no accidental allocation-heavy design.
22. canonical specs match the final implementation.

Do not declare Vutcom complete merely because the syntax parses or a UI demo works.

Completion means the generic composition subsystem is correct end-to-end.