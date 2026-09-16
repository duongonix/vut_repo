
# Vut Module Resolver

## 1. Purpose

The module resolver maps Vut import syntax to actual modules and symbols.

It enforces:

```text
module paths
relative imports
source roots
visibility
selected imports
aliases
cycles
collisions
```

---

## 2. Module Model

Every:

```text
.vut
```

file is a module.

Folders form namespaces.

No module declaration inside source is required.

---

## 3. Source Root

Project modules resolve relative to:

```text
src/
```

Package source roots are supplied to the compiler by VPM.

Compiler itself does not download packages.

---

## 4. Module ID

Resolved modules receive:

```text
ModuleId
```

Do not repeatedly identify modules only by path strings.

---

## 5. Module Table

Conceptually:

```text
ModuleTable
ModuleId -> ModuleData
```

Module data may include:

```text
SourceId
filesystem path
logical module path
symbols
dependencies
```

---

## 6. Absolute Import

From source root:

```vut
import a.b.c
```

resolves to conceptually:

```text
src/a/b/c.vut
```

---

## 7. Relative Import

Custom Vut semantics:

```text
.      current
..     up 1
...    up 2
....   up 3
```

Examples:

```vut
import .c
import ..utils
import ...math
```

---

## 8. Relative Normalization

Relative paths must normalize against importing module's directory.

Do not treat leading dots as Python semantics automatically unless identical to Vut's explicitly defined rules.

---

## 9. Root Escape

If a relative import escapes the source/package root:

```text
E3006
```

Do not allow filesystem path traversal outside compiler source roots.

---

## 10. Whole Module

```vut
import math
```

binds module name:

```text
math
```

or final segment for nested import according to language rules.

---

## 11. Alias

```vut
import math as m
```

binds:

```text
m
```

to resolved module.

---

## 12. Selected Symbols

```vut
import math at add, plus
```

resolves selected public symbols.

Private symbols must fail.

---

## 13. Private Symbols

Anything beginning `_` is private to its module.

Resolver should derive visibility centrally.

Do not add separate `pub` flags from syntax that does not exist.

---

## 14. Import Collision

If imports introduce the same binding:

```text
E3003
```

Use aliases to resolve collisions where possible.

---

## 15. Module Discovery

Do not scan the entire filesystem repeatedly for every import.

Build or lazily maintain a module lookup/index per source root.

Cache normalized module paths.

Local module discovery must map `mod.vut` to its containing directory's module
path. For each local module path, candidate precedence is:

```text
1. <base>/<name>.vut
2. <base>/<name>/mod.vut
```

The selected candidate is cached as the canonical module identity. Filesystem
enumeration order must not affect the result.

---

## 16. Package Modules

VPM supplies dependency source roots associated with package names.

Example dependency:

```text
nam/abc/libs/math
```

is imported as:

```vut
import math
```

Compiler should not care whether source originated from GitHub/GitLab/registry.

---

## 17. Package Namespace

Only the package's exposed final name enters normal import syntax.

Provider metadata must remain outside source language.

The exposed package namespace is the dependency table key. If that key matches
a local top-level module namespace, module graph construction reports `E3007`
and does not apply local/package shadowing.

---

## 18. Module Graph

Construct explicit directed graph:

```text
ModuleId -> imported ModuleIds
```

Use it for:

```text
cycle detection
compile ordering
incremental invalidation
parallel scheduling
```

---

## 19. Circular Imports

Vut v1 rejects circular imports.

Example:

```text
a -> b -> c -> a
```

Diagnostic should show full cycle.

---

## 20. Symbol Tables

Each module should have its own symbol table.

Conceptually:

```text
ModuleSymbols
name -> SymbolId
```

Public/private status belongs to symbol metadata.

---

## 21. Two-Phase Resolution

For robust cross-reference handling, implementation may use:

```text
1. declaration collection
2. body/name resolution
```

This allows functions/types declared later in a module to be known where language semantics permit it.

Exact declaration-order rules should remain explicit.

---

## 22. Name Resolution Scopes

Resolver must distinguish:

```text
module scope
function scope
block scope
method self
parameters
locals
imports
```

Do not implement all names in one flat map.

---

## 23. Shadowing

Exact shadowing policy must follow finalized semantic rules.

Do not silently forbid/allow new forms if not specified.

---

## 24. Unknown Names

Unknown local/global symbols:

```text
E2001
```

Module-not-found:

```text
E3001
```

Selected symbol missing:

```text
E3002
```

Keep these distinct.

---

## 25. Suggestions

Name/module typo suggestions may use edit distance or similar techniques.

Only suggest plausible nearby names.

Do not perform expensive global suggestion searches on every successful lookup.

---

## 26. Path Types

Filesystem handling must use:

```rust
Path
PathBuf
```

Do not concatenate filesystem paths with `/` strings.

Logical module paths should use separate structured representation.

---

## 27. Security

Normalize source-root traversal safely.

Never allow malicious module names to access arbitrary host files.

---

## 28. Incremental Support

Cache:

```text
module identity
imports
symbol exports
graph edges
```

Invalidate dependents when public module shape changes.

---

## 29. Tests

Method declarations arrive from the AST only in canonical
`fn Type.method(...)` form. Resolver method tables use a structured receiver
`SymbolId` plus method name key, validate that the receiver resolves to a named
type, inject `self` only for methods, and diagnose duplicates within the same
receiver namespace. Same-named methods on different receiver types do not
collide. Because extension methods are deferred, a method receiver must resolve
to a type declared in the same module.

Test:

```text
absolute imports
nested imports
relative imports
relative root escape
aliases
selected imports
private selected symbol
missing module
missing symbol
collisions
cycles
dependency package roots
Windows path behavior
Unix path behavior
```

---

## 30. Rules

1. Every `.vut` file is a module.
2. Folders form namespaces.
3. Source roots are explicit.
4. VPM provides dependency roots.
5. Compiler never performs package network access.
6. Relative-dot semantics follow Vut rules exactly.
7. Module IDs replace repeated path-string identity.
8. Module graph is explicit.
9. Circular imports are rejected.
10. Resolver must be incremental-friendly.
