# VPM — Vut Package Manager

## 1. Purpose

VPM is the official project and package manager for Vut.

Executable:

```text
vpm
```

VPM manages:

- projects
- dependencies
- package installation
- version resolution
- lockfiles
- package caches
- project builds
- project execution
- testing
- formatting
- linting
- documentation
- package workflows

The Vut compiler itself remains focused on compilation.

---

## 2. Responsibilities

VPM owns:

```text
project management
dependency management
package resolution
package downloading
package caching
lockfiles
project workflow
registry/provider access
```

The compiler owns:

```text
lexing
parsing
type checking
module resolution over provided roots
code generation
native compilation
```

---

## 3. Project Creation

Create a project:

```powershell
vpm new hello
```

Expected structure:

```text
hello/
├── src/
│   └── main.vut
├── tests/
├── vpm.toml
├── vpm.lock
└── .gitignore
```

VPM may initialize additional files as the ecosystem evolves.

---

## 4. Initialize Existing Directory

```powershell
vpm init
```

This initializes a Vut project in the current directory.

VPM must not silently overwrite unrelated existing files.

---

## 5. Project Manifest

Project configuration lives in:

```text
vpm.toml
```

Example:

```toml
[package]
name = "hello"
version = "0.1.0"

[dependencies]
math = "1.2.0"
```

Detailed format:

```text
specs/vpm/manifest.md
```

---

## 6. Lockfile

Resolved dependencies are stored in:

```text
vpm.lock
```

The lockfile provides reproducible dependency resolution.

Detailed format:

```text
specs/vpm/lockfile.md
```

---

## 7. Add Dependency

Default registry package:

```powershell
vpm add math
```

This is equivalent to requesting:

```powershell
vpm add math@latest
```

VPM resolves the latest stable version and writes a concrete version into project metadata.

---

## 8. Exact Version

```powershell
vpm add math@1.2.0
```

VPM resolves exactly:

```text
1.2.0
```

if available.

---

## 9. Self-Hosted GitHub Package

GitHub is the default self-host provider.

Syntax:

```powershell
vpm add nam/abc/math
```

Interpretation:

```text
owner:        nam
repository:   abc
package path: math
package name: math
```

Exact version:

```powershell
vpm add nam/abc/math@1.2.0
```

---

## 10. Deep Package Path

Packages may exist deeper inside a repository.

Example:

```powershell
vpm add nam/abc/libs/math
```

Interpretation:

```text
owner:        nam
repository:   abc
package path: libs/math
package name: math
```

---

## 11. Repository Is Not a Package

A repository itself can never be a Vut package.

A package must be a subdirectory inside a repository.

Therefore self-host syntax requires at minimum:

```text
owner/repository/package
```

Do not interpret:

```text
owner/repository
```

as a package.

---

## 12. GitLab Provider

GitLab uses an explicit provider prefix.

```powershell
vpm add gitlab:nam/abc/math
```

Exact version:

```powershell
vpm add gitlab:nam/abc/math@1.2.0
```

Provider architecture must allow additional providers in the future without rewriting dependency-resolution core logic.

---

## 13. Default Registry

The default VPM registry is hosted at the GitHub repository:

```text
duongonix/vpm
```

Conceptual structure:

```text
vpm/
├── math/
│   ├── 0.1.0/
│   ├── 0.1.1/
│   └── 1.0.0/
├── json/
│   ├── 1.0.0/
│   └── 1.1.0/
└── http/
    └── 0.5.0/
```

A package name maps to a folder in this repository.

---

## 14. Version Directories

Each package version is a subdirectory named:

```text
<semver>
```

There is no `v` prefix.

Examples:

```text
0.1.0
1.0.0
1.2.3
```

Each version directory contains a complete package.

---

## 15. Latest Resolution

These are equivalent:

```powershell
vpm add math
vpm add math@latest
```

VPM:

1. lists version directories
2. filters valid semantic versions
3. parses versions semantically
4. excludes prerelease versions for stable `latest`
5. selects the highest stable version

---

## 16. Semantic Ordering

Version selection must use semantic-version ordering.

Given:

```text
0.9.0
0.10.0
1.0.0
```

latest is:

```text
1.0.0
```

Do not compare version directory names lexicographically.

Use a proven semantic-version library.

---

## 17. Prerelease

Given:

```text
1.2.0
2.0.0-alpha.1
```

stable `latest` resolves to:

```text
1.2.0
```

Explicit prerelease installation is allowed:

```powershell
vpm add math@2.0.0-alpha.1
```

if that version exists.

---

## 18. Remove Dependency

```powershell
vpm remove math
```

VPM updates:

```text
vpm.toml
vpm.lock
```

Unused package cache data does not necessarily need to be deleted immediately.

---

## 19. Install

```powershell
vpm install
```

VPM reads project metadata and lockfile and ensures required dependencies exist locally.

When a valid lockfile entry exists, installation should prefer the locked version/source/revision rather than resolving a new version.

---

## 20. Update

```powershell
vpm update
```

Updates dependencies according to the project's allowed version/update rules.

Exact version-constraint semantics must be defined before advanced range updates are implemented.

The lockfile is regenerated or updated accordingly.

---

## 21. Dependency Tree

```powershell
vpm tree
```

Displays the resolved dependency graph.

Conceptual output:

```text
app 0.1.0
├── math 1.2.0
└── json 1.1.0
    └── core-utils 0.4.0
```

---

## 22. Outdated

```powershell
vpm outdated
```

Displays dependencies for which newer compatible or latest versions are available according to the configured update policy.

This operation may require provider network access.

---

## 23. Build

```powershell
vpm build
```

Workflow:

```text
read project
    ↓
resolve/install dependencies
    ↓
construct source roots
    ↓
invoke Vut compiler
```

VPM does not duplicate compiler implementation.

---

## 24. Run

```powershell
vpm run
```

Workflow:

```text
resolve project
    ↓
resolve dependencies
    ↓
compile
    ↓
execute
```

Program arguments may use:

```powershell
vpm run -- arg1 arg2
```

---

## 25. Check

```powershell
vpm check
```

Performs project semantic checking without requiring final executable generation when the compiler architecture supports it.

This is a VPM workflow command even though the compiler frontend performs the actual checking.

---

## 26. Test

```powershell
vpm test
```

Runs project tests.

Test conventions are defined in:

```text
specs/17-testing.md
```

---

## 27. Format

```powershell
vpm fmt
```

Formats Vut source according to the canonical formatter.

Detailed behavior:

```text
specs/18-formatter-linter.md
```

---

## 28. Lint

```powershell
vpm lint
```

Runs Vut lint rules.

Lint warnings must remain separate from compiler correctness errors.

---

## 29. Documentation

```powershell
vpm doc
```

Generates or displays project/package documentation according to the documentation tooling contract.

---

## 30. Clean

```powershell
vpm clean
```

Removes project build artifacts and appropriate local project caches.

It must not blindly delete the entire global VPM package store.

---

## 31. Search

The intended VPM workflow includes:

```powershell
vpm search <query>
```

For the GitHub-folder registry model, search may inspect registry metadata/index information.

A scalable registry index strategy may be introduced later.

Search implementation must not require changing package installation semantics.

---

## 32. Package Information

```powershell
vpm info math
```

may display information such as:

```text
name
available versions
latest stable version
package metadata
source
```

Exact presentation may evolve.

---

## 33. Publishing Commands

```text
vpm publish                      publish to the default registry (duongonix/vpm)
vpm publish alice/vut-packages   publish to a self-hosted registry
vpm publish --dry-run            validate + report without submitting
```

The package name comes from `[package].name`; it is never supplied again on the
command line.

Publication is a review-gated, source-only flow:

```text
validate manifest + layout + version + native build source
→ submit a change/PR to the target registry for review
→ trusted CI builds native artifacts per target
→ trusted CI uploads artifacts and generates native-artifacts.toml
```

The publisher must not submit prebuilt binaries or a publisher-authored
`native-artifacts.toml` (that file is registry/CI-controlled).

Provider and authentication are abstracted (GitHub is not privileged); the
submission credential is read from the environment and never written to the
manifest, lockfile, or package source.

`vpm login` and `vpm yank` remain reserved until their semantics are finalized.

Do not invent a centralized registry server merely to implement these commands.

---

## 34. Package Name

Package name is the final package-path segment.

Examples:

```text
nam/abc/math       -> math
nam/abc/libs/math  -> math
```

The package's manifest must agree with this name.

---

## 35. Dependency Name Collision

A project cannot contain two dependencies exposing the same package/import name.

Example:

```text
registry: math
github:nam/abc/math
```

must produce a collision.

VPM must not silently rename one package.

The dependency table key is the Vut import namespace. That namespace must also
not collide with a local top-level source module such as `src/http.vut` or
`src/http/mod.vut`. Users resolve this by aliasing the dependency key.

---

## 36. Global VPM Directory

Recommended structure:

```text
~/.vpm/
├── packages/
├── cache/
│   ├── downloads/
│   ├── registry/
│   └── build/
├── bin/
└── config.toml
```

Detailed storage behavior:

```text
specs/vpm/local-store.md
```

---

## 37. Source-Aware Package Store

Packages should retain source identity internally.

Example:

```text
~/.vut/packages/
├── registry/
│   └── math/
│       └── 1.2.0/
├── github/
│   └── nam/
│       └── abc/
│           └── math/
│               └── 1.2.0/
└── gitlab/
    └── nam/
        └── abc/
            └── math/
                └── 1.2.0/
```

Both remote and local version directories use the bare `<semver>` name with no
`v` prefix.

---

## 38. Global Store Ownership

The global package store is managed by VPM.

Users and build tools should treat installed package contents as read-only.

Projects must not modify dependency source in-place.

---

## 39. Download Cache

Downloaded archives/snapshots may be cached under:

```text
~/.vpm/cache/downloads/
```

Cache entries are disposable.

Deleting cache must not corrupt project metadata.

---

## 40. Registry Cache

Provider/registry metadata may be cached under:

```text
~/.vpm/cache/registry/
```

This may include:

- available versions
- metadata
- provider responses
- timestamps

Cache behavior must not cause stale data to violate explicit version requests.

---

## 41. Build Cache

Compiled dependency artifacts may be cached under:

```text
~/.vpm/cache/build/
```

Build cache is an implementation optimization.

It is not a public `.vutlib` distribution format.

---

## 42. No Project Dependency Folder

VPM should not create a Node.js-style:

```text
node_modules/
```

inside each project.

Dependencies are stored globally and referenced through resolved source roots.

This avoids unnecessary source duplication.

---

## 43. Reproducibility

`vpm.lock` should pin enough information to reproduce dependency resolution.

At minimum:

```text
name
version
source
revision and/or checksum
```

This also helps detect mutation of supposedly immutable published versions.

---

## 44. Provider Architecture

VPM should separate dependency resolution from hosting providers.

Conceptually:

```text
VPM Resolver
├── RegistryProvider
├── GithubProvider
└── GitlabProvider
```

Providers need operations conceptually similar to:

```text
list[path]
download(path)
```

Exact Rust traits/APIs are implementation details.

---

## 45. Network Layer

Use a proven Rust HTTP client/library.

Do not implement HTTP, TLS or GitHub/GitLab API transport manually.

Network functionality should be isolated from dependency-resolution logic.

---

## 46. Offline Behavior

When all required locked dependencies exist locally:

```powershell
vpm build
```

should not require unnecessary network access.

An explicit offline mode may be added.

Exact CLI flag is defined when implemented.

---

## 47. Diagnostics

VPM uses the visual principles defined in:

```text
specs/09-errors-diagnostics.md
```

Example:

```text
error: package `math` version `1.4.0` was not found

source:
  github:duongonix/vpm/math

requested:
  1.4.0

available:
  1.2.0
  1.3.0
```

Do not fabricate source-code frames for package-manager errors.

---

## 48. Atomic Manifest Updates

Commands such as:

```text
vpm add
vpm remove
vpm update
```

must avoid leaving partially written manifests or lockfiles after failure.

Updates should use safe/atomic file replacement where practical.

---

## 49. VPM Principles

VPM follows these principles:

1. VPM manages projects and packages.
2. `vut` remains focused on compilation.
3. Default registry uses the GitHub-folder model.
4. Repositories themselves are not packages.
5. Packages live inside repository subdirectories.
6. Versions live inside bare `<semver>` directories.
7. Each version contains a complete package.
8. `latest` selects the highest stable semantic version.
9. Explicit prerelease versions are allowed.
10. Installed packages are stored globally.
11. No `node_modules`-style project dependency directory is required.
12. Lockfiles provide reproducibility.
13. Package source identity is preserved.
14. Dependency namespace collisions are errors.
15. Provider architecture remains modular.
16. Proven Rust libraries should handle HTTP, SemVer, TOML and similar infrastructure.
17. Package-manager failures use clear structured diagnostics.

This document defines the high-level behavior of VPM.
