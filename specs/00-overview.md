# Vut Overview

## 1. Purpose

Vut is a compiled, statically typed programming language designed to combine:

- concise and readable syntax
- strong compile-time guarantees
- aggressive type inference
- predictable behavior
- simple project and package management
- high native performance
- clear compiler diagnostics

Vut aims to provide syntax with the simplicity of high-level languages while retaining the characteristics required for efficient native compilation.

Example:

```vut
data User:
  name: str
  age: int

fn User.greet():
  print("Hello " + self.name)

user = User(
  name: "Ha",
  age: 20
)

user.greet()
```

---

## 2. Core Goals

Vut is designed around the following primary goals.

### 2.1 Simple syntax

Common operations should require minimal boilerplate.

```vut
name = "Vut"
age = 20
```

Explicit declarations such as:

```text
let
var
const
public
private
export
```

are avoided when the same information can be expressed clearly through existing syntax or naming rules.

### 2.2 Static typing

Vut is statically typed.

Types are known and checked during compilation whenever possible.

```vut
age = 20
```

The compiler infers the type of `age`.

The following must fail:

```vut
age = 20
age = "twenty"
```

### 2.3 Type inference

Users should not need to repeat types when the compiler can infer them.

```vut
name = "Ha"
count = 10
active = true
```

Explicit types remain available:

```vut
name: str = "Ha"
count: int = 10
active: bool = true
```

### 2.4 Native performance

Vut targets native compilation and high runtime performance.

The language design should avoid features that inherently require expensive dynamic behavior unless the programmer explicitly requests them.

Dynamic typing is available through:

```vut
value: dyn = 10
```

but is not the default.

### 2.5 Predictability

Vut should prefer explicit compile-time behavior over surprising implicit runtime behavior.

Examples:

- variable types cannot silently change
- collections are homogeneous unless explicitly dynamic
- missing required `data` fields are compile errors
- private symbols cannot be imported
- interface compatibility is checked by the compiler
- invalid indentation is a compile error

### 2.6 High-quality diagnostics

Compiler diagnostics are a first-class part of Vut.

Every source-related compile error must clearly identify:

- error code
- error description
- file
- line
- column
- source code
- exact error span
- relevant related locations
- expected and found values/types when appropriate
- useful help when a reliable suggestion is available

Example:

```text
error[E1003]: type mismatch
  --> src/main.vut:4:12
   |
4  | age: int = "20"
   |            ^^^^ expected `int`, found `str`
   |
   = expected: int
   = found:    str
```

---

## 3. Source Files

Vut source files use:

```text
.vut
```

Example project:

```text
hello/
├── src/
│   ├── main.vut
│   ├── user.vut
│   └── utils.vut
├── tests/
├── vpm.toml
└── vpm.lock
```

Every `.vut` file is automatically a module.

---

## 4. Language Style

Vut uses indentation-based blocks.

```vut
if active:
  print("active")
```

Vut does not use braces for blocks.

Vut does not require semicolons.

```vut
name = "Ha"
age = 20
print(name)
```

The canonical formatter uses two spaces per indentation level.

---

## 5. Programming Model

Vut does not use traditional class inheritance.

The primary programming model consists of:

```text
data
functions
methods
interfaces
composition
modules
```

State is represented using `data`.

```vut
data Counter:
  value: int = 0
```

Behavior can be attached through methods.

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Abstraction is represented through interfaces.

```vut
interface Printable:
  print()
```

Types satisfy interfaces structurally.

No explicit `impl` or `implements` declaration is required.

---

## 6. No Classes or Inheritance

Vut does not provide class-based inheritance.

Do not introduce language constructs such as:

```text
class
extends
inherits
```

Reuse should primarily use composition.

```vut
data Position:
  x: float
  y: float

data Player:
  name: str
  position: Position
```

This keeps data relationships explicit and avoids inheritance hierarchies.

---

## 7. Visibility Model

Vut is public by default.

Identifiers beginning with `_` are private to their module.

Public:

```vut
fn connect():
  ...
```

Private:

```vut
fn _connect_internal():
  ...
```

Vut does not use:

```text
pub
public
private
export
```

The same rule applies to fields, methods, types and other module-level symbols.

---

## 8. Constant Model

ALL-CAPS bindings are constants.

```vut
MAX_SIZE = 100
PI = 3.14159
```

A constant cannot be reassigned.

No `const` keyword is required.

---

## 9. Expression-Oriented Design

Vut allows expressions to produce values where this improves clarity.

Example:

```vut
max = if a > b:
  a
else:
  b
```

Function bodies may return their final expression implicitly.

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Explicit `return` remains available for early exits.

---

## 10. Control Flow Philosophy

Vut intentionally keeps looping syntax small.

There is no `while`.

All looping uses `for`.

Infinite:

```vut
for:
  work()
```

Conditional:

```vut
for active:
  work()
```

Iterable:

```vut
for item in items:
  print(item)
```

Range:

```vut
for i in 0..10:
  print(i)
```

This reduces the number of control-flow constructs while retaining equivalent capabilities.

---

## 11. Collection Philosophy

Vut avoids square-bracket collection syntax.

List:

```vut
numbers = @[1, 2, 3]
```

Named structured data:

```vut
data Config:
  host: str
  port: int

config = Config(host: "localhost", port: 8080)
```

Vut does not have anonymous records or tuples.

Named structured data should use `data`.

---

## 12. Module Philosophy

Every `.vut` file is a module.

Import a module:

```vut
import math
```

Alias:

```vut
import math as m
```

Import selected symbols:

```vut
import math at add, plus
```

Relative imports follow Vut's own path model:

```vut
import .utils
import ..shared
import ...core
```

Module behavior is defined in:

```text
specs/07-modules-imports.md
```

---

## 13. Tooling

Vut uses two primary command-line tools:

```text
vut
vpm
```

### `vut`

`vut` is the compiler-facing CLI.

Its primary commands are:

```text
vut run
vut build
```

The compiler CLI should remain intentionally small.

### `vpm`

`vpm` is the Vut Package Manager.

It handles:

- project creation
- dependency management
- package resolution
- installation
- updates
- project builds
- project execution
- testing
- formatting
- linting
- documentation
- package-related workflows

---

## 14. Package Philosophy

Vut packages are distributed as source code.

There is no public `.vutlib` binary package format.

Packages contain Vut source that is compiled as part of the dependency graph.

VPM may maintain internal compilation caches, but those caches are implementation details.

---

## 15. Package Hosting

The default VPM registry is hosted through the VPM registry repository.

Packages may also be self-hosted through supported providers such as GitHub and GitLab.

A repository itself is not a Vut package.

A package must exist inside a repository subdirectory.

Example:

```text
nam/abc/
└── math/
    ├── 0.1.0/
    ├── 0.1.1/
    └── 1.0.0/
```

Install:

```text
vpm add nam/abc/math@0.1.1
```

Each version directory contains a complete package.

---

## 16. Version Philosophy

Package versions use semantic versioning.

Version directories use:

```text
<major>.<minor>.<patch>
```

Examples:

```text
0.1.0
0.1.1
1.0.0
2.3.4
```

A version can be selected explicitly:

```text
vpm add math@1.2.0
```

No version and `@latest` are equivalent:

```text
vpm add math
vpm add math@latest
```

VPM resolves the highest stable semantic version.

Published versions should be immutable.

---

## 17. Compiler Architecture Principle

The language specification defines behavior independently from compiler implementation.

The compiler is expected to have clearly separated stages such as:

```text
source
  ↓
lexer
  ↓
parser
  ↓
AST
  ↓
semantic analysis
  ↓
type checking
  ↓
interface checking
  ↓
lowering
  ↓
optimization
  ↓
code generation
  ↓
native output
```

The exact implementation architecture is defined under:

```text
specs/compiler/
```

---

## 18. Separation of Specifications

Top-level specifications describe language and tooling behavior.

Examples:

```text
01-language-syntax.md
02-type-system.md
05-control-flow.md
07-modules-imports.md
14-vpm.md
```

Compiler specifications describe implementation architecture.

Examples:

```text
compiler/lexer.md
compiler/parser.md
compiler/type-checker.md
compiler/codegen.md
```

Implementation details must not accidentally redefine language semantics.

---

## 19. Design Principles

Future Vut features should follow these principles:

1. Keep syntax small and consistent.
2. Prefer static verification.
3. Infer information when it is unambiguous.
4. Avoid unnecessary annotations.
5. Avoid hidden behavior.
6. Prefer composition over inheritance.
7. Keep runtime overhead explicit.
8. Make errors understandable.
9. Keep tooling consistent with the language.
10. Avoid adding multiple syntaxes for the same operation without a strong reason.
11. Do not sacrifice language consistency merely to imitate another language.
12. Prefer existing proven compiler/runtime libraries where they fit the implementation.

---

## 20. Non-Goals for the Initial Version

The initial Vut implementation does not need to prioritize:

- class inheritance
- exceptions
- macro systems
- complex metaprogramming
- async syntax (now specified separately for a single-thread foundation; see `specs/async/`)
- advanced concurrency primitives
- unstable experimental syntax

These features must not block implementation of the core language.

`async fn` and `await` are specified as a post-MVP single-thread foundation.
Full concurrency (threads, channels, multi-thread schedulers, parallel
execution) remains a non-goal for the initial version.

---

## 21. Specification Authority

Files inside `specs/` define the intended behavior of Vut.

When implementation behavior conflicts with an approved specification, the implementation should be treated as incorrect unless the specification itself is intentionally revised.

Changes to core syntax or semantics should first update the relevant specification before compiler behavior is changed.

This document provides the high-level contract for the Vut language and ecosystem.
