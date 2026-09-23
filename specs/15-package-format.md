# Vut Package Format

## 1. Purpose

This document defines the physical and logical format of Vut packages.

The package model is intentionally source-based.

A Vut package:

- is a directory
- contains source code
- contains `vpm.toml`
- has a semantic version
- is independently complete
- is compiled from source
- is not distributed as `.vutlib`

---

## 2. Fundamental Repository Rule

A repository itself is never a Vut package.

A package must exist as a subdirectory inside a repository.

Valid:

```text
repository/
└── math/
```

The repository root itself is not interpreted as package `math`.

For self-hosted packages, the minimum identity is:

```text
owner/repository/package
```

---

## 3. Version Directory

Inside a package directory, each published version is stored in its own directory.

Format:

```text
<semantic-version>
```

There is no `v` prefix.

Example:

```text
math/
├── 0.1.0/
├── 0.1.1/
└── 1.0.0/
```

---

## 4. Complete Version

Every version directory is a complete independent package.

Example:

```text
math/
└── 1.2.0/
    ├── vpm.toml
    ├── src/
    │   ├── mod.vut
    │   ├── vector.vut
    │   └── matrix.vut
    ├── tests/
    ├── README.md
    └── LICENSE
```

A version directory must not contain only differences from another version.

---

## 5. No Delta Versions

Do not design:

```text
1.2.0/
  full source

1.2.1/
  only changed files
```

`1.2.1` must contain everything required to use `1.2.1` independently.

This makes versions:

- portable
- immutable
- cacheable
- easy to verify

---

## 6. Required Manifest

Every package version must contain:

```text
vpm.toml
```

Example:

```toml
[package]
name = "math"
version = "1.2.0"
```

The manifest is required for package validation.

---

## 7. Package Name Validation

Given path:

```text
math/1.2.0/
```

the manifest must contain:

```toml
[package]
name = "math"
```

Invalid:

```toml
[package]
name = "json"
version = "1.2.0"
```

VPM must reject the package.

---

## 8. Version Validation

Given directory:

```text
math/1.2.0/
```

the manifest must contain:

```toml
version = "1.2.0"
```

Invalid:

```toml
version = "1.3.0"
```

The directory and manifest version must agree.

---

## 9. Version Directory Naming

Remote and local package version directories are named with the bare semantic
version, with no `v` prefix.

Example:

```text
1.2.0
```

The manifest version is the same bare semantic version:

```toml
version = "1.2.0"
```

Not:

```toml
version = "v1.2.0"
```

---

## 10. Semantic Versioning

Versions follow semantic versioning.

Canonical stable format:

```text
major.minor.patch
```

Examples:

```text
0.1.0
1.0.0
1.2.3
10.4.21
```

Prerelease versions may also exist:

```text
2.0.0-alpha.1
2.0.0-beta.2
2.0.0-rc.1
```

Corresponding directories:

```text
2.0.0-alpha.1
2.0.0-beta.2
2.0.0-rc.1
```

---

## 11. Invalid Version Directories

Directories that do not parse as supported semantic versions must not be considered package versions.

Example:

```text
latest/
old/
backup/
v1/
v1.2/
1/
1.2/
version1.2.0/
```

These must be ignored by version discovery or reported appropriately when explicitly requested.

---

## 12. Source Directory

Package Vut source lives under:

```text
src/
```

Example:

```text
math/1.2.0/
└── src/
    ├── mod.vut
    ├── vector.vut
    └── matrix.vut
```

The package source root is:

```text
src/
```

All Vut source of a package lives below `src/`.

---

## 13. Library Entry

The only public library entry is:

```text
src/mod.vut
```

Example:

```text
src/
├── mod.vut
├── parser.vut
└── lexer.vut
```

When `src/mod.vut` exists, it exposes the package namespace root and may
coordinate the public package API according to module visibility/import rules.

It does not use an `export` keyword.

`src/mod.vut` is optional; a package may be CLI-only.

`src/lib.vut` is not a library entry and has no special meaning. It is treated
as an ordinary module named `lib`. There is no deprecated alias.

---

## 14. Application Entry

Command-line entry points are declared explicitly in `vpm.toml` with `[[bin]]`:

```toml
[[bin]]
name = "math"
path = "src/bin/math.vut"
```

Each `path`:

- is relative to the package root
- must live below `src/`
- must end in `.vut`
- may appear in any subdirectory of `src/` (conventionally `src/bin/`)

Bin names are Vut identifiers and must be unique. A package may be library-only,
CLI-only, or both. There is no implicit `src/main.vut` application entry.

See `specs/vpm/manifest.md` for the manifest schema.

---

## 15. Tests

Packages may contain:

```text
tests/
```

Example:

```text
math/1.2.0/
├── src/
└── tests/
    ├── vector.vut
    └── matrix.vut
```

Testing semantics are defined in:

```text
specs/17-testing.md
```

---

## 16. README

A package version may contain:

```text
README.md
```

It should document:

- purpose
- usage
- examples
- important compatibility information

README is recommended but not required for compilation.

Registry policy may require it for officially indexed packages later.

---

## 17. License

Packages may contain:

```text
LICENSE
```

VPM should preserve package license files when downloading/caching packages.

Registry policy may require license metadata in the future.

---

## 18. Package Dependencies

A package may declare dependencies in its own:

```text
vpm.toml
```

Example:

```toml
[dependencies]
core_math = "1.0.0"
```

Self-hosted dependency form may use structured metadata.

Exact manifest syntax is defined in:

```text
specs/vpm/manifest.md
```

---

## 19. Package Import Name

The default Vut import namespace is the package name. In a project manifest,
the dependency table key is the actual import namespace, allowing aliases such
as `webhttp` for package identity `http`.

For:

```text
nam/abc/math
```

package name:

```text
math
```

Source imports:

```vut
import math
```

Repository owner and repository name do not become source namespaces.

---

## 20. Deep Package Paths

Given:

```text
nam/abc/libs/math
```

the package name remains:

```text
math
```

The full path identifies the dependency source.

The final segment defines the package/import name.

---

## 21. Package Namespace Collision

These packages:

```text
nam/a/math
nam/b/math
```

both expose:

```text
math
```

They cannot coexist as normal dependencies in the same project.

A dependency import namespace must also not collide with a local top-level
module namespace such as `src/http.vut` or `src/http/mod.vut`.

VPM must report the collision.

It must not silently expose:

```text
math1
math2
```

or source-qualified imports.

---

## 22. Default Registry Format

Default registry:

```text
github.com/duongonix/vpm
```

Conceptual structure:

```text
vpm/
├── math/
│   ├── 0.1.0/
│   │   ├── vpm.toml
│   │   └── src/
│   └── 1.0.0/
│       ├── vpm.toml
│       └── src/
│
├── json/
│   └── 1.0.0/
│       ├── vpm.toml
│       └── src/
│
└── http/
    └── 0.5.0/
        ├── vpm.toml
        └── src/
```

Each first-level package directory follows the same version-directory rules as self-hosted packages.

---

## 23. Self-Hosted Repository Format

Example:

```text
github.com/nam/abc/
├── math/
│   ├── 1.0.0/
│   │   ├── vpm.toml
│   │   └── src/
│   └── 1.1.0/
│       ├── vpm.toml
│       └── src/
│
└── json/
    └── 2.0.0/
        ├── vpm.toml
        └── src/
```

One repository may therefore host multiple independent Vut packages.

---

## 24. Multiple Packages per Repository

This is valid:

```text
repository/
├── math/
│   └── 1.0.0/
└── json/
    └── 1.0.0/
```

Install independently:

```powershell
vpm add owner/repository/math@1.0.0
vpm add owner/repository/json@1.0.0
```

This is a fundamental reason repository root itself is not treated as a package.

---

## 25. Nested Package Paths

Packages may be organized into deeper directories.

Example:

```text
repository/
└── libs/
    ├── math/
    │   └── 1.0.0/
    └── json/
        └── 1.0.0/
```

Install:

```powershell
vpm add owner/repository/libs/math@1.0.0
```

Package name:

```text
math
```

---

## 26. Published Version Immutability

A published version must be treated as immutable.

Once:

```text
math/1.2.0
```

is published, its source must not be modified.

If a bug is discovered, publish:

```text
math/1.2.1
```

Do not modify:

```text
math/1.2.0
```

in place.

---

## 27. Mutation Detection

VPM should store enough information in `vpm.lock` to detect unexpected mutation.

Possible information:

```text
source revision
commit
content checksum
```

If locked package contents unexpectedly change, VPM should report the mismatch rather than silently accepting the changed source.

---

## 28. Source-Only Distribution

Vut packages are distributed as source.

A package contains:

```text
.vut source
manifest
documentation
tests
supporting package files
```

VPM/compiler compiles package source for the target project.

---

## 29. No `.vutlib`

Vut does not define `.vutlib` as a public package distribution format.

Do not publish:

```text
math.vutlib
```

as the canonical VPM package artifact.

This decision allows:

- target-specific compilation
- whole-program optimization
- interface checking
- source-level diagnostics
- simpler package portability

---

## 30. Internal Build Artifacts

VPM/compiler may internally cache:

```text
AST
HIR
IR
object files
compiled dependency output
metadata
```

These are caches.

They do not redefine the package format.

Users should not need to manually distribute them.

---

## 31. Package Completeness

A package version must contain everything required to compile that package except declared dependencies.

It must not rely on undeclared files from:

- sibling versions
- repository root
- unrelated package directories

For example:

```text
math/1.2.0/
```

must not silently read source from:

```text
math/1.1.0/
```

---

## 32. Repository-Level Files

A hosting repository may contain unrelated files:

```text
README.md
LICENSE
.github/
scripts/
docs/
```

These are repository files.

They are not automatically part of any Vut package version.

Package boundaries are explicit through package/version directories.

---

## 33. Package Validation

Before accepting a package version, VPM should validate at least:

1. version directory is valid
2. `vpm.toml` exists
3. package name matches path
4. manifest version matches directory
5. a `src/mod.vut` library entry or at least one `[[bin]]` entry exists
6. every declared `[[bin]].path` exists below `src/`
7. manifest is parseable
8. dependency declarations are valid

Invalid packages must produce clear diagnostics.

---

## 34. Path Safety

Downloaded packages must not be able to escape their destination through malicious paths.

VPM extraction/downloading must reject unsafe paths such as conceptual:

```text
../../outside
```

Package handling must defend against path traversal.

---

## 35. Symlinks

Symlink handling in downloaded packages must be conservative.

VPM must not allow package content to use symlinks to escape the package/store boundary.

Exact supported symlink policy should be defined by the local-store/security implementation.

---

## 36. Package Size and File Limits

VPM may impose reasonable safety limits for:

- package size
- number of files
- individual file size
- directory depth

Limits should prevent accidental or malicious resource exhaustion.

Exact limits are implementation/configuration policy.

---

## 37. Package Format vs Local Store

Remote format:

```text
math/
└── 1.2.0/
```

The local store uses the same bare `<semver>` version directory, so the layout
is identical and does not encode any source prefix.

Detailed local layout is defined in:

```text
specs/vpm/local-store.md
```

---

## 38. Package Identity

A fully resolved package identity conceptually includes:

```text
provider/source
repository
package path
package name
version
revision/checksum
```

Example:

```text
provider: github
owner: nam
repository: abc
path: libs/math
name: math
version: 1.2.0
revision: ...
```

The source import namespace remains only:

```text
math
```

---

## 39. Package Format Principles

The Vut package format follows these principles:

1. Repository root is never a package.
2. Packages live in repository subdirectories.
3. Versions live under package directories.
4. Version directories use bare `<semver>` with no `v` prefix.
5. Every version is complete and independent.
6. Every version contains `vpm.toml`.
7. Manifest name must match package path.
8. Manifest version must match version directory.
9. Package source lives under `src/`.
10. Packages are distributed as source.
11. The public library entry is `src/mod.vut`; `src/lib.vut` has no special meaning.
12. Command-line entry points are declared with `[[bin]]`.
13. `.vutlib` does not exist as a public distribution format.
14. Published versions are immutable.
15. Lockfiles may detect unexpected mutation.
16. One repository may contain multiple packages.
17. Nested package paths are supported.
18. Final path segment determines package/import name.
19. Package namespace collisions are rejected.
20. Package handling must prevent path traversal and unsafe extraction.

This document is normative for the Vut package format.
