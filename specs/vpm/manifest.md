# VPM Manifest

## 1. Purpose

This document defines the `vpm.toml` project and package manifest format.

Every Vut project managed by VPM uses:

```text
vpm.toml
```

The manifest describes:

* package metadata
* project metadata
* dependencies
* dependency sources
* exact dependency versions

VPM must parse TOML using a mature Rust TOML library.

Do not implement a custom TOML parser.

Recommended:

```text
serde
toml
toml_edit
```

Use `toml_edit` when modifying an existing manifest so comments and formatting can be preserved where practical.

---

## 2. Basic Manifest

Example:

```toml
[package]
name = "hello"
version = "0.1.0"

[dependencies]
math = "1.2.0"
json = "2.0.1"
```

---

## 3. Package Section

Required fields:

```toml
[package]
name = "hello"
version = "0.1.0"
```

`name` identifies the package.

`version` must be valid SemVer.

Use the Rust:

```text
semver
```

crate for validation.

Do not implement SemVer manually.

---

## 4. Package Name

Package names must follow VPM package naming rules.

A dependency's import namespace is the dependency table key. In the compact
form this is normally the package name.

Example:

```text
math
```

is imported in Vut as:

```vut
import math
```

The exact identifier validation should remain compatible with Vut module identifiers.

Aliased dependency form:

```toml
[dependencies.webhttp]
package = "http"
version = "1.2.0"
```

This imports as `webhttp` while preserving package identity `http`.

---

## 5. Package Version

Manifest versions do not contain the remote-directory `v` prefix.

Correct:

```toml
version = "1.2.0"
```

Incorrect:

```toml
version = "v1.2.0"
```

Remote storage may use:

```text
v1.2.0/
```

but the semantic version itself is:

```text
1.2.0
```

---

## 6. Registry Dependency

A dependency from the default registry uses the compact form:

```toml
[dependencies]
math = "1.2.0"
json = "2.1.0"
```

This means:

```text
source = default VPM registry
package = math
version = 1.2.0
```

---

## 7. Self-Hosted GitHub Dependency

Example:

```toml
[dependencies.math]
source = "nam/abc/math"
version = "1.2.0"
```

Interpretation:

```text
provider     GitHub
owner        nam
repository   abc
package path math
package name math
version      1.2.0
```

GitHub is the default provider when no provider prefix exists.

---

## 8. Deep GitHub Package Path

Example:

```toml
[dependencies.math]
source = "nam/abc/libs/math"
version = "1.2.0"
```

Interpretation:

```text
owner        nam
repository   abc
package path libs/math
package name math
```

---

## 9. GitLab Dependency

Example:

```toml
[dependencies.math]
source = "gitlab:nam/abc/math"
version = "1.2.0"
```

The explicit:

```text
gitlab:
```

prefix selects GitLab.

---

## 10. Concrete Versions

Normal manifests should store concrete resolved versions.

After:

```text
vpm add math
```

if latest resolves to:

```text
1.4.2
```

VPM should write:

```toml
[dependencies]
math = "1.4.2"
```

Do not normally write:

```toml
math = "latest"
```

---

## 11. Dependency Identity

The dependency table key is the package/import name.

Example:

```toml
[dependencies.math]
source = "nam/abc/libs/math"
version = "1.2.0"
```

The project sees the package as:

```text
math
```

and Vut imports it as:

```vut
import math
```

---

## 12. Namespace Collision

A project cannot contain two dependencies exposing the same package name.

Example:

```text
registry math
nam/abc/math
```

cannot both exist as `math`.

VPM must report an error.

Do not silently rename either package.

---

## 13. Manifest Validation

VPM must validate:

```text
required package fields
package name
SemVer
dependency names
dependency versions
dependency source syntax
duplicate package namespace
unsupported fields where critical
```

Errors must clearly identify:

```text
vpm.toml
relevant field
invalid value
expected format
```

---

## 14. Remote Manifest Validation

When downloading:

```text
math/v1.2.0/
```

its manifest must contain:

```toml
[package]
name = "math"
version = "1.2.0"
```

If remote path and manifest disagree, installation fails.

---

## 15. Manifest Editing

Commands such as:

```text
vpm add
vpm remove
vpm update
```

must modify manifests carefully.

Prefer preserving:

```text
comments
unrelated sections
reasonable formatting
user metadata
```

Do not regenerate the entire manifest unnecessarily.

---

## 16. Unknown Fields

Unknown metadata that is clearly non-critical may eventually be preserved.

Unknown fields that could affect:

```text
dependency resolution
build behavior
package identity
security
```

must not be silently ignored unless compatibility rules explicitly allow it.

---

## 16a. Native Artifacts

A project that declares `extern "C"` functions may link local native static
libraries through the `[native]` table:

```toml
[native]
libraries = ["native/libimage.lib"]
system_libraries = ["user32"]
```

* `libraries` entries are static library paths (`.lib`/`.a`) relative to the
  project root.
* `system_libraries` are platform system library names passed to the linker.

Remote artifact download, checksum verification, and dynamic libraries are
deferred. Their intended future manifest shape and linking semantics are
described by:

```text
specs/ffi/04-native-linking-and-runtime.md
```

---

## 17. Future Metadata

Potential future metadata may include:

```text
authors
license
description
repository
homepage
keywords
minimum Vut version
edition
build configuration
```

These fields are not defined by this specification yet.

Do not invent their semantics during implementation.

---

## 18. Rules

1. `vpm.toml` uses TOML.
2. Package name and version belong under `[package]`.
3. Versions use SemVer.
4. Manifest versions omit the `v` directory prefix.
5. Default-registry dependencies may use compact string syntax.
6. Self-hosted dependencies use structured dependency tables.
7. GitHub is the default self-host provider.
8. GitLab uses `gitlab:` prefix.
9. Dependency namespace is the final package name.
10. Namespace collisions are errors.
11. Resolved versions should be concrete.
12. Remote manifest identity must match package path/version.
13. VPM should preserve existing manifest formatting where practical.
14. TOML and SemVer must use proven libraries.
