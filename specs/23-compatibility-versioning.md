# Vut Compatibility and Versioning

## 1. Purpose

This document defines versioning and compatibility policy for:

* the Vut language
* the Vut compiler
* the Vut runtime
* the standard library
* VPM
* Vut packages
* package manifests
* lockfiles
* compiler caches

The goal is to allow Vut to evolve without making compatibility unpredictable.

---

## 2. Version Domains

Vut has several distinct version domains.

Conceptually:

```text
language version
compiler version
runtime ABI version
standard library version
VPM version
package version
manifest format version
lockfile format version
```

These versions may be related but must not be treated as identical concepts.

---

## 3. Compiler Version

The Vut compiler exposes its version through:

```text
vut --version
```

Example:

```text
vut 0.4.0
```

Compiler releases should follow semantic versioning where practical.

---

## 4. VPM Version

VPM has its own executable/tool version.

Conceptually:

```text
vpm 0.4.0
```

VPM and compiler releases may initially move together, but the architecture must not assume they can never diverge.

---

## 5. Language Stability Before 1.0

Before Vut reaches:

```text
1.0.0
```

the language may still contain breaking changes.

However, breaking changes must still be intentional and documented.

Do not treat pre-1.0 status as permission for arbitrary undocumented syntax changes.

---

## 6. Language Stability at 1.0

Once Vut reaches `1.0.0`, programs written against the stable language specification should not be broken casually.

Breaking language changes should require a major compatibility mechanism such as:

* major language version
* edition
* explicit migration

The exact edition mechanism may be introduced before Vut 1.0 if necessary.

---

## 7. Language Version vs Compiler Version

Compiler release version does not automatically equal language version.

A newer compiler may compile the same stable language version while adding:

* optimizations
* diagnostics
* targets
* tooling improvements

Therefore avoid embedding compiler implementation version directly into source syntax.

---

## 8. Edition Direction

Vut may introduce an edition mechanism in the future if syntax/semantic evolution requires it.

Conceptually:

```toml
[package]
edition = "2027"
```

This syntax is not yet locked.

Do not add edition metadata until its necessity and behavior are formally specified.

---

## 9. Backward Compatibility

A newer compiler should aim to compile programs accepted by an older compiler within the same stable language compatibility line.

Example:

```text
compiler 1.1
```

should normally compile valid:

```text
Vut 1.x source
```

unless the source relied on explicitly deprecated/removed behavior under a documented policy.

---

## 10. Forward Compatibility

An older compiler is not expected to understand syntax introduced by a newer language version.

When possible, the compiler should produce a clear diagnostic such as:

```text
this project requires a newer Vut compiler
```

rather than generic parse failures if version metadata allows detection.

---

## 11. Package Versions

Vut packages use semantic versioning.

Format:

```text
major.minor.patch
```

Examples:

```text
0.1.0
1.0.0
2.3.4
```

Remote directories add:

```text
v
```

Example:

```text
v2.3.4/
```

Manifest:

```toml
version = "2.3.4"
```

---

## 12. Package SemVer Meaning

Recommended interpretation:

### Patch

```text
1.2.3 -> 1.2.4
```

Bug fixes and compatible changes.

### Minor

```text
1.2.0 -> 1.3.0
```

Backward-compatible features.

### Major

```text
1.0.0 -> 2.0.0
```

Breaking API changes.

Before package `1.0.0`, SemVer pre-1.0 rules apply.

---

## 13. Package Immutability

A published package version is immutable.

If:

```text
math/v1.2.0
```

has been published, its contents must not be changed.

A fix becomes:

```text
math/v1.2.1
```

---

## 14. Source Revision

Even with semantic package versions, VPM may record source revision information.

Example:

```toml
[[package]]
name = "math"
version = "1.2.0"
source = "registry:math"
revision = "63a9d..."
```

This protects against mutation of a supposedly immutable remote version.

---

## 15. Content Checksums

VPM may additionally record a content checksum.

Conceptually:

```text
checksum = "..."
```

The checksum validates downloaded package contents.

Exact algorithm and lockfile field are defined in VPM lockfile specifications.

Use a proven cryptographic hashing library.

Do not implement a custom checksum algorithm.

---

## 16. Dependency Compatibility

The initial VPM design prefers concrete package versions.

Example:

```toml
[dependencies]
math = "1.2.0"
```

This should be interpreted according to the manifest/resolver specification.

Do not automatically assume Cargo/npm-style range behavior unless version-range syntax is explicitly specified.

---

## 17. `latest` Is Not Persisted as Floating State

When:

```text
vpm add math
```

resolves:

```text
1.3.2
```

the project should store a concrete resolved version.

Do not persist:

```text
latest
```

as ordinary build behavior.

This prevents builds from changing when new remote versions appear.

---

## 18. Lockfile Compatibility

`vpm.lock` is machine-managed.

Users should not need to understand every internal field to build a project.

VPM should detect unsupported lockfile format versions and produce a clear diagnostic.

---

## 19. Lockfile Format Version

The lockfile should eventually contain an explicit format/schema version.

Conceptually:

```toml
lock-version = 1
```

Exact field name is defined in:

```text
specs/vpm/lockfile.md
```

This allows VPM to evolve the lockfile safely.

---

## 20. Reading Older Lockfiles

Newer VPM versions should attempt to read older compatible lockfile formats.

If automatic upgrade is safe, VPM may regenerate/update the lockfile.

If not, VPM must explain why migration is required.

---

## 21. Newer Lockfiles on Older VPM

If an older VPM sees a lockfile format it cannot understand, it should stop with a clear error.

Do not silently ignore unknown critical fields.

Example:

```text
error: unsupported VPM lockfile format

project requires:
  lock format 3

this VPM supports:
  lock format 1-2
```

---

## 22. Manifest Compatibility

`vpm.toml` syntax is part of the VPM compatibility contract.

New optional fields may generally be added compatibly.

Changes that reinterpret existing fields require stronger compatibility consideration.

---

## 23. Unknown Manifest Fields

Policy should distinguish:

```text
unknown optional metadata
unknown critical behavior
```

VPM may ignore explicitly non-critical future metadata when safe.

It must not ignore unknown fields that could materially change dependency/build behavior.

Exact schema rules belong to `specs/vpm/manifest.md`.

---

## 24. Runtime ABI Version

Compiler-generated code and the Vut runtime must agree on an ABI contract.

Runtime ABI may cover:

```text
str
list
dyn
interfaces
allocation
runtime calls
```

Compiler must not link incompatible runtime ABI versions silently.

---

## 25. Runtime ABI Mismatch

Example diagnostic:

```text
error[E9003]: incompatible Vut runtime ABI

compiler expects:
  runtime ABI 4

found:
  runtime ABI 3
```

The normal toolchain installation should prevent this situation where possible.

---

## 26. Runtime Internal ABI

Runtime ABI is primarily an internal toolchain contract.

It is not automatically a stable third-party C ABI.

Third-party native integration uses the FFI specification instead.

---

## 27. Standard Library Compatibility

The standard library is part of the language ecosystem compatibility surface.

A compiler should use a compatible standard library version.

Core standard-library API breakage should be treated carefully after Vut 1.0.

---

## 28. Compiler and Standard Library Pairing

The toolchain may ship compiler/runtime/std versions known to work together.

Conceptually:

```text
Vut Toolchain
├── compiler
├── runtime
└── std
```

Users should not normally need to manually match these components.

---

## 29. VPM and Compiler Compatibility

VPM invokes compiler functionality.

VPM must detect gross incompatibility where relevant.

Example:

```text
project requires Vut compiler >= X
```

Exact manifest constraints for compiler versions are deferred until needed.

---

## 30. Package Minimum Language Version

Packages may eventually declare the minimum Vut language/compiler compatibility required.

Conceptual future metadata:

```toml
[package]
vut = "..."
```

Exact syntax is not yet locked.

Do not invent dependency-range semantics for this field before specification.

---

## 31. Compiler Cache Compatibility

Compiler caches are not stable public artifacts.

They may be invalidated when:

* compiler version changes
* runtime ABI changes
* optimization strategy changes
* target changes
* source changes
* relevant flags change

Cache invalidation is expected and must be safe.

---

## 32. Never Trust Old Cache Blindly

Cache entries require fingerprints sufficient to determine compatibility.

Conceptual cache key inputs:

```text
compiler version
target
build mode
source hash
dependency hash
relevant flags
IR/cache schema version
```

Invalid cache should be discarded and rebuilt.

---

## 33. Build Artifact Compatibility

Internal object files and cached IR are not package distribution contracts.

Do not assume cached artifacts produced by compiler version A are consumable by compiler version B unless explicitly validated.

---

## 34. Native Binary Compatibility

Native executables follow their target OS/platform ABI.

Vut does not guarantee that one compiled executable runs on different operating systems or CPU architectures.

Cross-platform portability comes from source packages and recompilation.

---

## 35. Source Compatibility

Source compatibility means a newer compatible compiler accepts existing source with the same intended semantics.

This is distinct from binary ABI compatibility.

Vut prioritizes source compatibility for normal packages.

---

## 36. Standard Library Deprecation

Before removing a stable standard-library API, tooling should ideally:

1. mark it deprecated
2. issue warning
3. provide migration guidance
4. retain it for a documented transition period
5. remove it only under compatible versioning policy

---

## 37. Language Deprecation

Syntax or semantics should not be deprecated casually.

If a stable language feature must be replaced, the compiler should provide targeted migration diagnostics where practical.

---

## 38. Deprecated Warning

A future warning may use a code such as:

```text
W0xxx or another assigned range
```

Exact code must be added to:

```text
specs/22-error-codes.md
```

before use.

---

## 39. Feature Introduction

New language syntax should follow this process:

```text
1. define behavior
2. update normative specs
3. update grammar
4. define diagnostics
5. implement lexer/parser
6. implement semantics
7. add tests
8. update formatter
9. update examples/docs
```

Codex must not implement syntax first and document it afterward.

---

## 40. Breaking Language Change Process

A breaking language change should update all affected specifications.

Potentially:

```text
01-language-syntax.md
02-type-system.md
03-data-model.md
04-functions-methods.md
05-control-flow.md
06-interfaces.md
07-modules-imports.md
08-memory-model.md
21-grammar.md
22-error-codes.md
23-compatibility-versioning.md
```

Relevant examples/tests must also be updated.

---

## 41. Package Format Evolution

Changes to package layout must preserve clarity between:

```text
repository
package path
version directory
package source
```

The fundamental currently locked rule remains:

```text
repository is not a package
package is a subdirectory
version is a v<semver> subdirectory
```

Changing this would be a major VPM/package-format compatibility decision.

---

## 42. Provider Compatibility

GitHub/GitLab provider API changes must not alter Vut package identity semantics.

Provider adapters isolate external service behavior from resolver logic.

---

## 43. Error Code Compatibility

Published compiler error codes are compatibility identifiers for documentation and tooling.

Do not reuse:

```text
E1003
```

for a different semantic error later.

Wording and source presentation may improve.

---

## 44. Diagnostic Format Compatibility

Human diagnostic rendering may improve between releases.

Machine-readable diagnostic schemas require explicit versioning if breaking changes occur.

Editors should not parse colored human terminal text as a stable machine protocol.

---

## 45. Formatter Compatibility

Within a language compatibility line, formatter changes may alter layout but must not alter semantics.

Formatting changes should remain deterministic.

Large style changes should be introduced intentionally to avoid unnecessary repository-wide churn.

---

## 46. Package APIs

Package authors are responsible for semantic versioning of their own public APIs.

For example, removing a public function generally requires a major version increment after 1.0.

Adding compatible functionality generally uses a minor version.

Bug fixes generally use patch versions.

---

## 47. Private Package APIs

Symbols beginning with `_` are private and not part of the supported external module API.

Package authors may change private internals without treating those changes as public API breakage, subject to their own internal needs.

---

## 48. Structural Interfaces and Compatibility

Because interface satisfaction is structural, adding a required method to an existing public interface can break existing concrete types.

Therefore this may be a breaking API change.

Example:

Old:

```vut
interface Reader:
  read() -> bytes
```

New:

```vut
interface Reader:
  read() -> bytes
  close()
```

Existing types satisfying the old interface may fail under the new version.

Package authors must account for this in SemVer decisions.

---

## 49. Data Compatibility

Adding a required field to a public `data` type can break existing construction calls.

Example:

Old:

```vut
data User:
  name: str
```

New:

```vut
data User:
  name: str
  age: int
```

This is potentially breaking.

Adding a field with a compatible default may be less disruptive but must still consider observable API behavior.

---

## 50. Function Compatibility

Breaking examples include:

```text
removing a public function
renaming a function
changing parameter type
changing parameter count
changing return type incompatibly
making public API private
```

These should inform package major-version decisions.

---

## 51. No Hidden Auto-Migration

Compiler/VPM should not silently rewrite project source to newer syntax during ordinary build commands.

Migration tools may be introduced separately.

Normal:

```text
vpm build
```

must not unexpectedly modify source files.

---

## 52. Reproducible Builds

Given:

```text
same project source
same vpm.lock
same dependency contents
same compatible compiler/toolchain
same target/configuration
```

Vut should aim for reproducible semantic builds.

Byte-for-byte binary reproducibility should also be pursued where backend/toolchain behavior permits.

---

## 53. Version Comparison Libraries

Compiler/VPM implementation must use a mature SemVer library.

Do not implement version parsing/comparison manually.

This applies to:

* package versions
* prerelease ordering
* future toolchain constraints

---

## 54. Compatibility Diagnostics

When compatibility prevents an operation, diagnostics should state:

```text
what component is incompatible
required version
found version
how to resolve it
```

Example:

```text
error: incompatible Vut compiler

project requires:
  Vut >= 1.4.0

found:
  Vut 1.2.0

help:
  update the Vut toolchain
```

Do not merely report:

```text
version mismatch
```

---

## 55. Compatibility Principles

1. Compiler, language, runtime, VPM and packages are distinct version domains.
2. Package versions use SemVer.
3. Published package versions are immutable.
4. Lockfiles pin concrete package resolution.
5. Runtime ABI compatibility must be validated.
6. Compiler caches are disposable implementation artifacts.
7. Source compatibility is a primary goal after Vut 1.0.
8. Breaking language changes require explicit migration/versioning policy.
9. Stable diagnostic codes must not be repurposed.
10. Machine-readable formats require schema/version compatibility.
11. Provider changes must not redefine package identity.
12. New syntax must be specified before implementation.
13. Ordinary builds must not silently rewrite source.
14. Compatibility failures require actionable diagnostics.
15. Proven SemVer libraries must be used rather than custom implementations.

This document defines the compatibility and versioning policy for the Vut language and ecosystem.
