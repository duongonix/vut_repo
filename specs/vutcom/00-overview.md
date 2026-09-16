# Vutcom — Overview

> **Superseded.** The trailing `:` composition syntax is now owned by
> `specs/receiver/` (Receiver Functions). This document is kept for historical
> reference only and must not drive implementation. See
> `specs/receiver/00-overview.md`.

## 1. Purpose

Vutcom is Vut's generic typed declarative composition system.

Vutcom is NOT:

- a UI framework
- a renderer
- a Virtual DOM
- a widget system
- a tree serialization format
- a state management system
- a routing system
- a build system
- a workflow engine

Instead, Vutcom provides a language-level mechanism for libraries to define typed composition domains.

Possible domains include:

- UI
- Build
- Route
- Workflow
- Config
- Test
- Deployment
- Query
- Scene
- Document

The compiler provides:

- composition domains
- `vutcom(D)`
- composition syntax
- domain type safety
- child composition
- control-flow composition
- ownership integration
- generic lowering
- optimization opportunities

Libraries define:

- domain semantics
- primitive composition operations
- consumers
- execution strategies
- rendering where applicable
- state/reactivity where applicable

---

## 2. Core Model

A library may define a composition domain:

```vut
composition UI
```

Another library may define:

```vut
composition Build
```

These introduce distinct composition domains.

Their corresponding types are:

```vut
vutcom(UI)
vutcom(Build)
```

These are different types.

Example:

```vut
fn Home() -> vutcom(UI):
  ...

fn Release() -> vutcom(Build):
  ...
```

Therefore:

```text
Home()    : vutcom(UI)
Release() : vutcom(Build)
```

There is no implicit mixing between domains.

---

## 3. Opaque Composition Value

`vutcom(D)` is an opaque composition value belonging to domain `D`.

Example:

```vut
page: vutcom(UI) = Home()
```

Users must NOT depend on an internal representation such as:

```vut
page.nodes
page.children
page.props
```

The language specification MUST NOT define Vutcom as:

```text
Node
Tree
Virtual DOM
list(Node)
map(str, dyn)
```

The compiler/runtime is free to internally use:

- specialized composition code
- static metadata
- captures
- stack values
- compact frames
- direct consumer operations
- compiler-generated structures
- other optimized representations

provided observable Vut semantics remain unchanged.

---

## 4. Composition Functions

A composition-producing function is still an ordinary Vut function.

Do NOT introduce special declarations such as:

```text
component
widget
view
@Composable
```

Canonical form:

```vut
fn UserCard(user: User) -> vutcom(UI):
  Card():
    Text(value = user.name)
```

The return type identifies the function as producing a composition.

Normal Vut functionality remains available:

- local variables
- local functions
- callbacks
- function calls
- type inference
- generics
- if/else
- for
- match where valid
- result propagation where valid

Vutcom must integrate into Vut instead of creating a second programming language inside Vut.

---

## 5. Generic Composition

UI is only one possible application.

UI:

```vut
fn Home() -> vutcom(UI):
  Column():
    Text(value = "Hello")

ui.run(Home())
```

Build:

```vut
fn Release() -> vutcom(Build):
  Pipeline():
    Compile()
    Test()
    Package()

build.execute(Release())
```

Routing:

```vut
fn Routes() -> vutcom(Route):
  Route(path = "/")
  Route(path = "/users")

server.serve(Routes())
```

All use the same Vutcom language mechanism.

The compiler MUST NOT contain domain-specific concepts such as:

```text
Button
Text
Column
Widget
Route
Pipeline
Compile
DOM
Renderer
```

---

## 6. Consumer Model

A Vutcom consumer is an ordinary typed library function.

Conceptually:

```vut
fn run(root: vutcom(UI)):
  ...
```

or:

```vut
fn execute(root: vutcom(Build)):
  ...
```

Usage:

```vut
ui.run(Home())
build.execute(Release())
```

`run`, `execute`, `serve`, `inspect`, etc. are NOT compiler intrinsics.

A domain may have multiple consumers:

```vut
plan = Release()

build.execute(plan)
build.inspect(plan)
build.graph(plan)
```

The composition model must therefore not be tied to one renderer or execution strategy.

---

## 7. Domain Safety

Composition domains are statically checked.

Given:

```vut
fn Home() -> vutcom(UI):
  ...

fn Release() -> vutcom(Build):
  ...
```

Valid:

```vut
ui.run(Home())
build.execute(Release())
```

Invalid:

```vut
ui.run(Release())
```

Expected diagnostic:

```text
expected `vutcom(UI)`
found `vutcom(Build)`
```

Domain identity must be represented by the type system.

Do NOT implement domains using runtime strings.

---

## 8. Cross-Domain Composition

Implicit cross-domain composition is forbidden.

A:

```text
vutcom(UI)
```

cannot automatically become:

```text
vutcom(Route)
```

If a library intentionally accepts another composition domain, it must declare that explicitly through an ordinary typed parameter.

Example:

```vut
Route(
  path = "/",
  page = Home()
)
```

may be valid if `Route` explicitly accepts:

```vut
page: vutcom(UI)
```

This is ordinary typed composition passing, not implicit domain conversion.

---

## 9. Performance Principle

Vutcom should follow:

```text
compile-time work ↑
runtime work ↓
allocations ↓
dynamic lookup ↓
```

The compiler should be free to:

- inline composition functions
- specialize generic composition
- eliminate intermediate `vutcom(D)` values
- constant-fold static arguments
- eliminate dead composition
- use escape analysis
- optimize captures
- stack allocate where safe
- generate static metadata
- specialize consumer paths

Vutcom MUST NOT require:

- heap allocation per component
- generic Node allocation
- runtime string component lookup
- mandatory tree construction
- mandatory Virtual DOM
- mandatory full-tree diffing

---

## 10. Core Boundary

Vutcom core owns:

```text
composition domains
vutcom(D)
composition blocks
children
domain checking
control-flow composition
composition lowering
ownership integration
generic optimization
```

Vutcom core does NOT own:

```text
UI state
recomposition
renderer
layout
DOM
Virtual DOM
event loop
routing table
build dependency graph
workflow scheduler
HTTP
graphics
```

Those belong to libraries.

---

## 11. Fundamental Rule

The central rule of Vutcom is:

> Vutcom is a generic typed declarative composition mechanism. The compiler provides composition, domain safety and optimization. Libraries define what a composition means and how it is consumed.

This boundary must remain intact throughout implementation.