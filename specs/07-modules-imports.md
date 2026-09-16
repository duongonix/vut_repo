# Vut Modules and Imports

## 1. Purpose

This document defines:

- modules
- source roots
- absolute imports
- relative imports
- module aliases
- selected imports
- visibility
- private symbols
- import collisions
- circular imports
- package imports

Vut's module system maps directly to files and directories.

---

## 2. File as Module

Every `.vut` source file is automatically a module.

Example:

```text
src/math.vut
```

defines module:

```text
math
```

No declaration such as:

```text
module math
```

is required.

---

## 3. Directory Namespaces

Directories form namespaces.

Example:

```text
src/
└── app/
    └── user.vut
```

corresponds to:

```text
app.user
```

A module path is derived from its location relative to the source root.

A directory becomes an importable module object only when it contains:

```text
mod.vut
```

For an import segment `name`, local source resolution uses this deterministic
candidate order:

```text
1. name.vut
2. name/mod.vut
```

If both exist, `name.vut` wins. This is not an ambiguity error. Submodule
traversal may still descend into `name/` for paths such as `name.child`.

---

## 4. Source Root

For normal application projects, the primary source root is:

```text
src/
```

Example:

```text
project/
├── src/
│   ├── main.vut
│   └── math.vut
├── vpm.toml
└── vpm.lock
```

Absolute project imports resolve from `src/`.

---

## 5. Basic Import

Import an entire module:

```vut
import math
```

Use:

```vut
math.add()
math.PI
```

The module name becomes a binding in the current module.

---

## 6. Module Alias

A module may be imported with an alias.

```vut
import math as m
```

Use:

```vut
m.add()
```

The original module binding `math` is not additionally introduced by this import unless imported separately.

---

## 7. Selected Imports

Specific public symbols may be imported using `at`.

```vut
import math at add, plus
```

Use directly:

```vut
add()
plus()
```

The canonical forms are:

```vut
import math
import math as m
import math at add, plus
```

---

## 8. `as` and `at`

For Vut v1, a single import must not combine `as` and `at`.

Do not introduce syntax such as:

```vut
import math as m at add
```

Use separate forms when necessary.

---

## 9. Absolute Imports

Given:

```text
src/
├── main.vut
└── a/
    └── b/
        ├── c.vut
        └── d.vut
```

Inside `d.vut`:

```vut
import a.b.c
```

resolves to:

```text
src/a/b/c.vut
```

Absolute import paths are resolved from the source root.

---

## 10. Default Module Binding

For:

```vut
import a.b.c
```

the default local binding is the final module segment:

```text
c
```

Usage:

```vut
c.run()
```

If a different binding is desired:

```vut
import a.b.c as parser
```

Usage:

```vut
parser.run()
```

---

## 11. Relative Imports

Vut uses leading dots for relative imports.

The rule is:

```text
.       current directory
..      up 1 directory
...     up 2 directories
....    up 3 directories
```

This rule is intentionally Vut-specific.

---

## 12. Current Directory Import

Given:

```text
src/
└── a/
    └── b/
        ├── c.vut
        └── d.vut
```

Inside `d.vut`:

```vut
import .c
```

resolves to:

```text
src/a/b/c.vut
```

---

## 13. Parent Directory Import

Given:

```text
src/
└── a/
    ├── utils.vut
    └── b/
        └── d.vut
```

Inside `d.vut`:

```vut
import ..utils
```

resolves to:

```text
src/a/utils.vut
```

---

## 14. Multiple Parent Levels

Given:

```text
src/
├── math.vut
└── a/
    └── b/
        └── d.vut
```

Inside `d.vut`:

```vut
import ...math
```

means:

```text
up two directories
then resolve math.vut
```

Result:

```text
src/math.vut
```

---

## 15. Relative Descending Paths

Relative imports may move upward and then descend.

Example:

```vut
import ..shared.ui
```

Alias:

```vut
import ..shared.ui as ui
```

Selected symbols:

```vut
import ..shared.ui at button, input
```

---

## 16. Relative Import Boundary

A relative import must not move above the permitted source/package root.

If:

```vut
import ....something
```

attempts to escape the source root, compilation must fail.

The diagnostic should identify:

- current module
- requested relative path
- source root boundary

---

## 17. Imports Are Compile-Time Declarations

Imports are resolved during compilation.

Imports are module-level declarations.

Do not allow imports inside functions.

Invalid:

```vut
fn run():
  import math
```

The compiler must report that imports are only valid at module scope.

---

## 18. Public Symbols

Symbols are public by default.

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Other modules may import `add`.

```vut
import math at add
```

---

## 19. Private Symbols

Identifiers beginning with `_` are private to their module.

Example:

```vut
fn _fast_add(a: int, b: int) -> int:
  a + b
```

Another module must not import:

```vut
import math at _fast_add
```

This is a compile error.

---

## 20. Private Module Members

Private behavior applies consistently to:

- functions
- variables
- constants if their names begin with `_`
- data types
- fields
- methods
- interfaces
- enums
- aliases
- other module-level declarations

Example:

```vut
data _InternalState:
  value: int
```

This type cannot be referenced from another module through normal imports.

---

## 21. Private Access Within the Same Module

Private symbols remain accessible inside their defining module.

Example:

```vut
fn _parse():
  ...

fn load():
  _parse()
```

This is valid.

Privacy is module-scoped, not function-scoped.

---

## 22. Selected Import Validation

Given:

```vut
import math at add, plus
```

the compiler must verify that:

- module `math` exists
- symbol `add` exists
- symbol `plus` exists
- both symbols are public
- local bindings do not collide

Each invalid symbol should receive a precise diagnostic.

---

## 23. Import Collision

Two imports must not introduce the same local binding accidentally.

Invalid:

```vut
import math at add
import vector at add
```

because both introduce:

```text
add
```

The compiler must report the collision.

Possible correction:

```vut
import math as math
import vector as vector
```

or select different symbols.

---

## 24. Module Binding Collision

This must also be detected:

```vut
import a.math
import b.math
```

because both default to local binding:

```text
math
```

Use aliases:

```vut
import a.math as amath
import b.math as bmath
```

---

## 25. Collision With Local Declarations

Imported bindings must not silently overwrite local declarations.

Example:

```vut
fn add():
  ...

import math at add
```

This must produce a name collision error.

Binding resolution must remain deterministic.

---

## 26. Missing Module

Example:

```vut
import math.vector
```

when no corresponding module exists.

Diagnostic concept:

```text
error[E3001]: module not found
  --> src/main.vut:1:8
   |
1  | import math.vector
   |        ^^^^^^^^^^^ module `math.vector` was not found
```

The compiler may display relevant searched paths.

---

## 27. Missing Symbol

Example:

```vut
import math at unknown
```

when `unknown` is not defined.

The compiler must identify:

```text
module: math
symbol: unknown
```

and may suggest similarly named public symbols.

---

## 28. Circular Imports

Circular imports are rejected in Vut v1.

Example:

```text
a.vut imports b
b.vut imports c
c.vut imports a
```

The compiler must report the cycle.

Example:

```text
error[E3010]: circular module dependency

a -> b -> c -> a
```

The diagnostic should reference relevant import locations when possible.

---

## 29. Module Resolution Before Type Checking

Imports must be resolved sufficiently early for the compiler to know:

- accessible symbols
- module identities
- imported declarations
- dependency relationships

Implementation details belong to:

```text
specs/compiler/module-resolver.md
```

This document defines behavior, not internal compiler architecture.

---

## 30. Package Imports

Installed VPM packages enter the module resolver as package source roots.

After:

```text
vpm add math@1.2.0
```

Vut source uses:

```vut
import math
```

The package's installation source must not leak into Vut source syntax.

The user should not need:

```text
import github.duongonix.vpm.math
```

---

## 31. Self-Hosted Package Imports

After:

```text
vpm add nam/abc/math@1.2.0
```

the package name is the final package path segment:

```text
math
```

Vut imports it as:

```vut
import math
```

not:

```vut
import nam.abc.math
```

VPM resolves package identity before compiler module resolution.

---

## 32. Package Name Collision

A project must not contain two dependencies that expose the same top-level package name.

For example:

```text
registry math
github nam/abc/math
```

both expose:

```text
math
```

The project dependency resolver must reject this before compilation.

Vut must not silently rename package modules.

A local top-level module namespace and a dependency import namespace must not
have the same name. This is a compile-time project error; neither local source
nor package dependency may silently shadow the other.

The dependency table key is the import namespace. A dependency may avoid a
collision by using an alias key, for example:

```toml
[dependencies.webhttp]
package = "http"
version = "1.2.0"
```

---

## 33. Package Internal Imports

Inside a package, imports follow the same module rules as normal project source.

Example package:

```text
math/
└── src/
    ├── lib.vut
    └── vector.vut
```

Package source may use:

```vut
import .vector
```

or appropriate absolute package-local paths according to the package source root.

---

## 34. No `export`

Vut does not use an export declaration.

Do not add:

```text
export fn add()
export add
```

Visibility is determined by naming:

```text
add       public
_add      private
```

---

## 35. Namespace Principles

The module system follows these principles:

1. Every `.vut` file is a module.
2. Directories form namespaces.
3. `src/` is the normal source root.
4. Absolute imports resolve from a source root.
5. Relative imports use Vut's leading-dot model.
6. Imports are compile-time and module-scoped.
7. Symbols are public by default.
8. `_name` is private.
9. No `export` keyword exists.
10. Import collisions are compile errors.
11. Circular imports are rejected in Vut v1.
12. Package installation paths do not leak into source import syntax.
13. Package name is the imported top-level namespace.
14. Resolution must be deterministic.

This document is normative for Vut's module and import behavior.
