
# Vut Package Resolution

## 1. Purpose

This document defines how VPM resolves:

* package sources
* package versions
* `latest`
* registry packages
* GitHub packages
* GitLab packages
* transitive dependencies
* dependency conflicts
* lockfiles
* local cache/store reuse

Package resolution must be:

* deterministic
* reproducible
* SemVer-aware
* source-aware
* safe
* independent from compiler module resolution

---

## 2. Resolution Responsibilities

VPM performs package resolution.

The Vut compiler must not contact registries or hosting providers to resolve dependencies.

The workflow is:

```text
vpm
  ↓
read vpm.toml
  ↓
resolve package identities
  ↓
resolve versions
  ↓
resolve transitive dependencies
  ↓
generate/read vpm.lock
  ↓
install source
  ↓
provide source roots to compiler
```

---

## 3. Package Reference Forms

Default registry:

```text
math
math@latest
math@1.2.0
```

GitHub self-hosted:

```text
nam/abc/math
nam/abc/math@1.2.0
nam/abc/libs/math@1.2.0
```

GitLab:

```text
gitlab:nam/abc/math
gitlab:nam/abc/math@1.2.0
```

---

## 4. Default Registry Resolution

For:

```text
vpm add math
```

VPM resolves package directory:

```text
github.com/duongonix/vpm/math/
```

The resolver lists valid version directories under that package.

---

## 5. `latest`

These commands are equivalent:

```text
vpm add math
vpm add math@latest
```

`latest` means:

> highest available stable semantic version.

---

## 6. Version Directory Filtering

Given:

```text
v0.9.0/
v0.10.0/
v1.0.0/
v2.0.0-alpha.1/
backup/
latest/
test/
```

stable candidates are:

```text
0.9.0
0.10.0
1.0.0
```

Prerelease:

```text
2.0.0-alpha.1
```

is excluded from stable `latest`.

Malformed directories are ignored during automatic version discovery.

---

## 7. Semantic Version Ordering

Versions must be parsed with a proven SemVer implementation.

Do not sort versions lexicographically.

For:

```text
0.9.0
0.10.0
1.0.0
```

correct ordering is:

```text
0.9.0
0.10.0
1.0.0
```

and latest is:

```text
1.0.0
```

---

## 8. Explicit Version

For:

```text
vpm add math@1.2.0
```

VPM resolves exactly:

```text
math/v1.2.0/
```

If that directory does not exist, resolution fails.

VPM must not silently substitute:

```text
1.2.1
1.3.0
2.0.0
```

---

## 9. Explicit Prerelease

Explicit prerelease versions are allowed.

```text
vpm add math@2.0.0-alpha.1
```

resolves:

```text
math/v2.0.0-alpha.1/
```

if available.

---

## 10. Self-Hosted GitHub Resolution

For:

```text
vpm add nam/abc/math@1.2.0
```

VPM interprets:

```text
provider: github
owner: nam
repository: abc
package path: math
package name: math
version: 1.2.0
```

Remote package directory:

```text
github.com/nam/abc/math/v1.2.0/
```

---

## 11. Deep GitHub Package Path

For:

```text
vpm add nam/abc/libs/math@1.2.0
```

interpret:

```text
owner: nam
repository: abc
package path: libs/math
package name: math
```

Version directory:

```text
libs/math/v1.2.0/
```

---

## 12. Repository Root Is Not a Package

This is not a valid self-hosted package reference:

```text
nam/abc
```

Self-hosted GitHub references require at least:

```text
owner/repository/package
```

The resolver must never reinterpret repository root as a package.

---

## 13. GitLab Resolution

GitLab references require:

```text
gitlab:
```

Example:

```text
gitlab:nam/abc/math@1.2.0
```

Interpretation is otherwise equivalent to the GitHub package model.

---

## 14. Provider Abstraction

Package resolution core must not contain GitHub-specific logic everywhere.

Conceptually:

```text
PackageResolver
    │
    ├── RegistryProvider
    ├── GithubProvider
    └── GitlabProvider
```

Providers conceptually support operations such as:

```text
list_directory
read_file
download_directory
resolve_revision
```

Exact Rust APIs are implementation-specific.

---

## 15. Package Validation

After resolving a version directory, VPM validates:

```text
vpm.toml
package name
package version
source structure
```

For:

```text
math/v1.2.0/
```

manifest must contain:

```toml
[package]
name = "math"
version = "1.2.0"
```

Mismatch is a resolution error.

---

## 16. Dependency Manifest

A package may have its own dependencies.

Example:

```toml
[dependencies]
json = "1.1.0"
```

VPM recursively resolves those dependencies.

---

## 17. Resolution Graph

Dependencies form a graph.

Example:

```text
app
├── math 1.2.0
│   └── core 1.0.0
└── json 2.0.0
    └── core 1.0.0
```

The resolver should reuse the same compatible resolved package identity where appropriate.

---

## 18. Package Identity

Resolved package identity includes more than its name.

Conceptually:

```text
provider
repository/source
package path
package name
version
revision/checksum
```

Example:

```text
github
nam/abc
libs/math
math
1.2.0
63a9d...
```

---

## 19. Import Namespace

Despite full source identity, Vut source sees only:

```text
math
```

Example:

```vut
import math
```

The provider or repository path is not part of source syntax.

---

## 20. Dependency Name Collision

A project cannot resolve two distinct packages with the same exposed package name.

Example:

```text
registry math
github nam/abc/math
```

both expose:

```text
math
```

This is an error.

VPM must not silently rename either dependency.

The exposed dependency name is the manifest dependency key. The compiler must
also reject that key when it collides with a local top-level module namespace.

---

## 21. Transitive Name Collision

The same rule applies to transitive dependencies.

If two different resolved package identities require the same exposed package name and cannot resolve to one identity, resolution must fail.

The diagnostic should show the dependency paths causing the conflict.

Example:

```text
app
├── a
│   └── math -> registry math@1.0.0
└── b
    └── math -> github:nam/abc/math@1.0.0
```

---

## 22. Version Conflict

If dependencies require incompatible exact versions of the same package identity, VPM must not silently choose one.

Example:

```text
A requires math@1.0.0
B requires math@2.0.0
```

If Vut currently exposes one package namespace/version per package identity, resolution fails.

The diagnostic must show both dependency paths.

---

## 23. No Silent Multi-Version Namespace

VPM v1 must not silently install:

```text
math@1
math@2
```

and expose both under hidden rewritten names.

If multiple versions of the same package cannot coexist under the language module model, report a conflict.

---

## 24. Dependency Resolution Algorithm

The resolver should conceptually perform:

```text
1. read root manifest
2. normalize dependency references
3. resolve source identity
4. resolve requested version
5. validate package
6. read package dependencies
7. recursively resolve
8. detect collisions/conflicts
9. construct dependency graph
10. produce deterministic lockfile
```

Implementation may optimize this process.

---

## 25. Cycle Detection

Package dependency cycles must be detected.

Example:

```text
A -> B -> C -> A
```

VPM should reject cycles for v1 unless a future specification explicitly defines package-cycle behavior.

Diagnostic:

```text
error: circular package dependency

A@1.0.0
  -> B@1.0.0
  -> C@1.0.0
  -> A@1.0.0
```

---

## 26. Lockfile Priority

When `vpm.lock` exists and is valid, normal operations should prefer locked resolutions.

Example:

```text
vpm install
vpm build
vpm run
```

should not automatically resolve newer package versions merely because they appeared remotely.

---

## 27. Concrete Versions in Manifest

When:

```text
vpm add math
```

resolves:

```text
1.3.2
```

VPM should write a concrete version rather than:

```text
latest
```

Example:

```toml
[dependencies]
math = "1.3.2"
```

This prevents ordinary builds from silently changing dependencies.

---

## 28. Lockfile Data

Each lockfile package should include at minimum:

```text
name
version
source
revision and/or checksum
```

Example:

```toml
[[package]]
name = "math"
version = "1.3.2"
source = "registry:math"
revision = "63a9d..."
```

Exact format is defined in:

```text
specs/vpm/lockfile.md
```

---

## 29. Immutable Versions

Published package versions are expected to be immutable.

If a locked remote version changes unexpectedly, VPM should detect this using:

```text
revision
checksum
```

and report an error.

Do not silently trust changed content under the same version.

---

## 30. Local Store Reuse

Before downloading a resolved package, VPM checks the local global store.

Conceptually:

```text
~/.vpm/packages/
```

If the exact source identity/version/revision exists and verifies correctly, reuse it.

---

## 31. Offline Resolution

When all locked dependencies exist locally, operations such as:

```text
vpm build
vpm run
vpm test
```

should not require network access.

---

## 32. Metadata Cache

Available version lists and provider metadata may be cached.

Cache entries must not override an explicit remote refresh operation.

Cache corruption should be recoverable by discarding and refetching metadata.

---

## 33. Deterministic Graph

For identical:

```text
vpm.toml
vpm.lock
VPM version behavior
available locked package contents
```

the dependency graph should be deterministic.

Output ordering in `vpm tree` and lockfiles should also be deterministic.

---

## 34. Error Diagnostics

Package resolution errors should explain:

* requested package
* requested version
* source
* available versions where useful
* dependency path where relevant
* conflicting packages where relevant

Example:

```text
error: package `math` version `1.4.0` was not found

source:
  registry:math

requested:
  1.4.0

available:
  1.2.0
  1.3.0
```

---

## 35. Version Conflict Diagnostic

Example:

```text
error: conflicting versions for package `math`

required by:
  app -> renderer -> math@1.4.0
  app -> physics  -> math@2.0.0

Vut v1 requires one resolved version for a package namespace.
```

---

## 36. Resolution Security

The resolver must validate external data.

Do not trust:

* package folder names
* manifests
* API paths
* downloaded archives
* checksums supplied only by untrusted package content

Provider and package inputs must be treated as untrusted.

---

## 37. Existing Libraries

The VPM implementation should use established Rust crates for:

* SemVer parsing
* TOML parsing
* HTTP
* hashing
* archive handling
* URL/path handling
* graph traversal where useful

Do not implement SemVer comparison or HTTP/TLS manually.

---

## 38. Resolution Principles

VPM dependency resolution follows these principles:

1. VPM resolves packages before compilation.
2. `latest` selects highest stable SemVer.
3. Explicit versions resolve exactly.
4. Prereleases require explicit requests unless future policy says otherwise.
5. Repository roots are not packages.
6. Package path determines package identity.
7. Final path segment determines import name.
8. Provider logic is modular.
9. Transitive dependencies form a validated graph.
10. Namespace collisions are errors.
11. Incompatible package versions are not silently selected.
12. Lockfiles take priority during reproducible workflows.
13. Published versions are expected to be immutable.
14. Local verified packages should be reused.
15. Resolution should work offline when all locked packages are available.
16. Resolution results are deterministic.

---
