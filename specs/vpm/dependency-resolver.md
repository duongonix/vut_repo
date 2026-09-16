# VPM Dependency Resolver

## 1. Purpose

The dependency resolver converts project dependency declarations into an exact package graph.

Conceptually:

```text
vpm.toml
    ↓
Dependency Resolver
    ↓
Resolved Package Graph
    ↓
vpm.lock
```

---

## 2. Responsibilities

The resolver is responsible for:

```text
dependency identity
source selection
version resolution
transitive dependencies
namespace collision detection
cycle detection
provider interaction
lockfile generation input
```

It must not contain HTTP-provider-specific implementation details.

---

## 3. Package Identity

Use a structured identity.

Conceptually:

```text
PackageId
├── source
├── package_path
├── name
└── version
```

Self-hosted source additionally contains:

```text
provider
owner
repository
```

Do not represent package identity as one loosely parsed string throughout the resolver.

Parse once into structured types.

---

## 4. Registry Dependency

Input:

```toml
[dependencies]
math = "1.2.0"
```

becomes conceptually:

```text
RegistryPackage {
  name: math
  version: 1.2.0
}
```

---

## 5. GitHub Dependency

Input:

```toml
[dependencies.math]
source = "nam/abc/math"
version = "1.2.0"
```

becomes structured:

```text
provider = github
owner = nam
repository = abc
package_path = math
name = math
version = 1.2.0
```

---

## 6. Deep Path

Input:

```text
nam/abc/libs/math
```

means:

```text
owner = nam
repository = abc
package_path = libs/math
name = math
```

---

## 7. Repository Is Never Package

This is mandatory.

Invalid conceptual source:

```text
nam/math
```

if interpreted as:

```text
owner/repository
```

with no package subfolder.

Self-host package syntax requires at least:

```text
owner/repository/package
```

---

## 8. Dependency Graph

The resolver should construct an explicit graph.

Conceptually:

```text
App
├── math
│   └── core-utils
└── json
    └── core-utils
```

Equivalent dependency identities should be reused rather than represented as unrelated duplicate nodes.

---

## 9. Graph Algorithms

Cycle detection and topological ordering must use clear graph algorithms.

A mature crate such as:

```text
petgraph
```

may be used if it fits the architecture.

A compact custom ID-based graph is also acceptable if simpler and demonstrably appropriate.

Do not implement graph logic as scattered recursive command code.

---

## 10. Cycles

Dependency cycles must be rejected unless a future package model explicitly allows them.

Example:

```text
a -> b -> c -> a
```

Diagnostic should display the cycle.

---

## 11. Namespace Collision

Two dependencies cannot expose the same package/import name in one project.

Example:

```text
registry:math
github:nam/abc/math
```

is a collision.

Do not silently rename.

The dependency table key is the import namespace. That namespace must not
collide with a local top-level module namespace; aliases are represented by a
different dependency key.

---

## 12. Diamond Dependencies

Example:

```text
app
├── a
│   └── core 1.0.0
└── b
    └── core 1.0.0
```

The identical resolved package may be shared.

---

## 13. Conflicting Versions

Exact semantics for multiple incompatible versions of the same package in a dependency graph must be explicitly defined before implementation chooses behavior.

Do not automatically copy Cargo/npm behavior.

Until specified, the resolver should prefer strict deterministic behavior and must not invent aliasing/import semantics.

---

## 14. Lockfile Reuse

If the lockfile contains a valid package resolution consistent with the manifest, reuse it.

Do not query providers unnecessarily.

---

## 15. Fresh Resolution

When resolution is required:

```text
parse dependency
↓
select provider
↓
resolve version
↓
obtain package manifest
↓
discover transitive dependencies
↓
repeat
↓
validate graph
```

---

## 16. Determinism

The same dependency inputs and provider metadata must produce the same resolved graph.

Do not let hash iteration or request completion order alter resolution.

---

## 17. Separation From Download

Resolution and installation should remain conceptually separate.

Resolver decides:

```text
what packages are needed
```

Installer decides:

```text
how packages are obtained/stored
```

This separation enables:

```text
dry resolution
testing
offline mode
better caching
```

---

## 18. Resolver Testing

Tests must cover at least:

```text
single dependency
multiple dependencies
deep package path
GitHub dependency
GitLab dependency
transitive dependency
diamond dependency
cycle
namespace collision
missing version
invalid manifest identity
prerelease
lockfile reuse
```

Provider network access should be mockable.

---

## 19. Performance

Avoid repeatedly resolving identical package metadata.

Use caches keyed by structured package/source identity.

---

## 20. Rules

1. Resolver uses structured package IDs.
2. Repository itself is never a package.
3. Package must exist below repository root.
4. Resolver builds an explicit graph.
5. Cycles are rejected.
6. Project import namespace collisions are rejected.
7. Resolution is deterministic.
8. Resolver and downloader are separate responsibilities.
9. Lockfile resolution is reused where valid.
10. Unspecified version-conflict semantics must not be invented.
