# Vutcom — Language Specification

> **Superseded.** The trailing `:` composition syntax is now owned by
> `specs/receiver/` (Receiver Functions). This document is kept for historical
> reference only and must not drive implementation. See
> `specs/receiver/00-overview.md`.

## 1. Composition Domain Declaration

A composition domain is declared with:

```vut
composition UI
```

Examples:

```vut
composition UI
composition Build
composition Route
composition Workflow
```

Each declaration creates a distinct domain type usable by `vutcom[D]`.

The compiler must treat domains nominally.

Two separately declared domains are not interchangeable even if their libraries provide similar primitives.

---

## 2. Vutcom Type

Canonical syntax:

```vut
vutcom[D]
```

Examples:

```vut
vutcom[UI]
vutcom[Build]
vutcom[Route]
```

`D` must resolve to a valid composition domain.

Invalid:

```vut
vutcom[int]
vutcom[str]
vutcom[User]
```

unless those symbols are explicitly composition domains.

---

## 3. Composition-Producing Function

Functions use normal `fn` syntax.

Example:

```vut
fn Home() -> vutcom[UI]:
  Column():
    Text(value: "Hello")
```

No special component declaration syntax exists.

Do NOT introduce:

```text
component Home
widget Home
view Home
@Composable
```

---

## 4. Composition Call

A call participating in composition uses ordinary call syntax.

Example:

```vut
Text(value: "Hello")
```

A call with child composition uses a trailing `:` block:

```vut
Column():
  Text(value: "Hello")
```

Another example:

```vut
Button(onclick: click):
  Text(value: "Save")
```

The parser should not create UI-specific syntax.

It should recognize a generic call expression with an optional trailing composition block.

---

## 5. Function Call vs Data Constructor

Syntax such as:

```vut
Hello(a: 1)
```

may syntactically represent a normal function call or data constructor.

The parser should initially represent this as a generic call-like expression.

Resolution determines symbol kind.

Conceptually:

```text
Hello = function
→ function call

Hello = data constructor
→ data construction
```

Do NOT introduce artificial syntax such as:

```text
@Hello
<Hello>
new Hello
```

to distinguish them.

If Vut's namespace rules do not permit unambiguous resolution, reject conflicting declarations instead of making Vutcom syntax more complex.

---

## 6. Trailing Composition Block

The `:` after a compatible call introduces child composition.

Example:

```vut
Card(title: "Profile"):
  Text(value: "Nam")
  Button():
    Text(value: "Follow")
```

The block is a composition value of the same required domain.

Conceptually:

```text
Card(
  title: "Profile",
  children: <composition block>
)
```

This conceptual transformation does not require materializing an intermediate runtime object.

---

## 7. Children

A component accepts child composition by declaring an appropriate parameter.

Canonical direction:

```vut
fn Card(
  title: str,
  children: vutcom[UI]
) -> vutcom[UI]:
  Panel():
    Text(value: title)
    children
```

Usage:

```vut
Card(title: "Profile"):
  Text(value: "Nam")
```

The trailing block supplies the child composition.

No separate public type such as:

```text
children(UI)
ChildList(UI)
VNodeChildren
```

should be introduced for MVP.

Use `vutcom[D]` itself.

---

## 8. Leaf Components

A composition-producing function that does not accept children must reject a trailing composition block.

Example:

```vut
fn Text(value: str) -> vutcom[UI]:
  ...
```

Valid:

```vut
Text(value: "Hello")
```

Invalid:

```vut
Text(value: "Hello"):
  Button()
```

The compiler must report a clear error explaining that the callable does not accept child composition.

---

## 9. Multiple Children

A composition block may contain multiple child composition expressions.

Example:

```vut
Column():
  Text(value: "A")
  Text(value: "B")
  Button():
    Text(value: "C")
```

The implementation must not require the user to manually create a list.

Do NOT require:

```vut
Column(children: @[Text(...), Text(...)])
```

for normal declarative syntax.

---

## 10. Nested Components

Composition functions may call other composition functions.

Example:

```vut
fn Avatar(user: User) -> vutcom[UI]:
  Image(src: user.avatar)

fn UserCard(user: User) -> vutcom[UI]:
  Card():
    Avatar(user: user)
    Text(value: user.name)

fn Home(user: User) -> vutcom[UI]:
  Column():
    UserCard(user: user)
```

Nested composition must remain statically typed.

---

## 11. Named Arguments

Vutcom uses Vut's normal named argument system.

Example:

```vut
Button(
  text: "Save",
  enabled: true,
  onclick: click
)
```

Do NOT introduce a separate props syntax or property map for Vutcom.

---

## 12. Ordinary Logic

A composition-producing function may contain ordinary Vut code.

Example:

```vut
fn UserCard(user: User) -> vutcom[UI]:
  title = "$(user.name) - $(user.age)"

  fn click():
    out(user.name)

  Card():
    Text(value: title)

    Button(onclick: click):
      Text(value: "Open")
```

Vutcom must not restrict functions to declarative statements only.

---

## 13. If / Else

Normal Vut `if/else` is supported inside composition.

Example:

```vut
fn Home(logged_in: bool) -> vutcom[UI]:
  Column():
    if logged_in:
      Profile()
    else:
      Login()
```

Both branches must satisfy the composition domain expected at that location.

The compiler may lower this into an internal dynamic composition region.

No public `If` component is required.

---

## 14. For

Normal Vut `for` is supported inside composition.

Example:

```vut
fn UserList(users: list[User]) -> vutcom[UI]:
  Column():
    for user in users:
      UserCard(user: user)
```

The compiler may lower this into an internal repeated composition region.

No public `ForEach` component is required.

---

## 15. Match

Where ordinary Vut `match` is valid and each selected path produces compatible composition, it should be supported.

Example conceptually:

```vut
match state:
  loading:
    Loading()
  ready:
    Content()
  failed:
    ErrorView()
```

Match support must reuse normal Vut semantics rather than introducing a Vutcom-specific match language.

---

## 16. Callbacks

Normal Vut callbacks may be passed to composition functions.

One-line callback:

```vut
Button(
  onclick: () => save()
)
```

Multiline callback:

```vut
Button(
  onclick: fn():
    save()
    out("saved")
)
```

Vutcom must use Vut's existing callback/function semantics.

Do NOT introduce a Vutcom-specific event language.

---

## 17. Composition Value Expression

A `vutcom[D]` value may itself appear inside a compatible composition block.

Example:

```vut
fn Card(
  children: vutcom[UI]
) -> vutcom[UI]:
  Panel():
    children
```

The inserted value must belong to the expected domain.

---

## 18. Domain Boundary

Inside a composition expecting `vutcom[UI]`, inserting `vutcom[Build]` directly is invalid.

Example:

```vut
fn Release() -> vutcom[Build]:
  ...

fn Home() -> vutcom[UI]:
  Column():
    Release()
```

This must fail at compile time.

Explicit typed cross-domain parameters remain allowed.

---

## 19. Primitive Composition Operations

Every domain eventually needs termination points that are not recursively implemented as ordinary composition functions forever.

Libraries must therefore be able to define primitive composition operations for a domain.

Examples conceptually:

```text
UI:
  Text
  Image
  Container

Build:
  Command
  Copy
  Process

Route:
  RouteEntry
  Group
```

The exact internal declaration/lowering mechanism must remain generic.

The compiler may know:

```text
this is a primitive composition operation for domain D
```

but MUST NOT know:

```text
this is a Button
this is a Route
this is a Compile task
```

If a new declaration mechanism is necessary, design it generically and document it before implementation.

---

## 20. No Implicit Domain Conversion

The following must never happen automatically:

```text
vutcom[UI]
→ vutcom[Build]

vutcom[Build]
→ vutcom[Route]
```

Cross-domain usage requires an explicitly typed API boundary.

---

## 21. No Mandatory Tree Semantics

The source syntax may visually resemble a tree:

```vut
Column():
  Text()
  Button():
    Text()
```

This is declarative nesting syntax.

It does NOT mean the language promises that runtime memory contains a tree.

The observable semantics are composition order, domain correctness and library-defined primitive behavior.

Internal representation remains implementation-defined.

---

## 22. Future Composition Identity

Composition identity is a useful generic capability for future phases.

Potential uses include:

```text
UI       → state identity
Build    → task identity
Workflow → step identity
Route    → route identity
```

However, explicit key syntax and advanced identity semantics are NOT required for the initial MVP.

Do not introduce UI-specific `key` semantics into the core.

---

## 23. Examples

### UI

```vut
composition UI

fn UserCard(user: User) -> vutcom[UI]:
  Card():
    Text(value: user.name)

    Button(onclick: () => follow(user)):
      Text(value: "Follow")

fn Home(users: list[User]) -> vutcom[UI]:
  Column():
    Text(value: "Users")

    for user in users:
      UserCard(user: user)

fn main():
  ui.run(Home(load_users()))
```

### Build

```vut
composition Build

fn Release() -> vutcom[Build]:
  Pipeline():
    Compile(mode: "release")
    Test()
    Package(output: "dist")

fn main():
  build.execute(Release())
```

These must use the same generic compiler machinery.