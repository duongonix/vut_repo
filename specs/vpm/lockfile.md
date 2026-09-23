
# VPM Lockfile

## 1. Purpose

`vpm.lock` records the exact dependency resolution used by a Vut project.

Its goals are:

* reproducible builds
* exact dependency pinning
* source verification
* dependency graph recording
* protection against mutated remote versions

The lockfile is managed by VPM.

Users should not normally edit it manually.

---

## 2. Location

Project:

```text
project/
├── vpm.toml
├── vpm.lock
└── src/
```

---

## 3. Basic Entry

Current format (`lock-version = 2`):

```toml
lock-version = 2

[[package]]
name = "math"
version = "1.3.2"
source = "registry:math"
revision = "63a9d..."
checksum = "blake3-hex"
dependencies = ["core 1.0.0 registry:core"]

[[package.native]]
target = "x86_64-pc-windows-msvc"
url = "https://example.com/math/1.3.2/math.lib"
checksum = "sha256:..."
size = 123456
system_libraries = ["user32"]
```

* `checksum` is the BLAKE3 checksum of the pinned package source.
* `[[package.native]]` entries pin resolved native artifacts per target.
* `native` entries are omitted entirely when a package has no native source.
* Native artifacts use a trusted, registry/CI-provided SHA-256 expectations, not
  a self-computed value.

---

## 4. Required Identity

Each locked package should eventually identify at least:

```text
name
version
source
```

and where supported:

```text
revision
checksum
dependencies
```

---

## 5. Source Identity

Examples:

```text
registry:math
github:nam/abc/math
gitlab:nam/abc/math
```

Deep path:

```text
github:nam/abc/libs/math
```

The lockfile must preserve source identity.

Two packages with identical name/version but different sources are not assumed to be identical artifacts.

---

## 6. Exact Versions

The lockfile stores exact versions.

Never:

```text
latest
^1.2
~1.2
*
```

unless a future lockfile format explicitly introduces some different concept.

A resolved lock entry must be concrete:

```text
1.2.4
```

---

## 7. Revision

For Git-backed providers, VPM should record the revision used to obtain the package where practical.

Example:

```toml
revision = "63a9d..."
```

This allows VPM to detect a version directory that was modified after publication.

---

## 8. Checksum

VPM may record a deterministic content checksum.

The checksum must be calculated from a canonical package-content definition specified by the implementation/package specification.

Do not hash unstable metadata accidentally.

Use a proven hashing crate.

Preferred candidate for VPM-owned content hashing:

```text
blake3
```

Do not implement hashing manually.

---

## 9. Dependency Graph

When transitive dependencies are supported, lock entries should record resolved dependency identities.

Conceptually:

```toml
dependencies = [
  "json 1.2.0 registry:json",
  "core-utils 2.0.1 github:nam/abc/core-utils"
]
```

Exact serialization format must avoid ambiguity when identical names can theoretically originate from different sources internally.

The resolver owns dependency identity.

---

## 10. Deterministic Ordering

Lockfile output must be deterministic.

Do not depend on Rust `HashMap` iteration order.

Packages and dependency entries should use a stable canonical ordering.

---

## 11. Atomic Writes

Updating `vpm.lock` should use safe atomic replacement where practical:

```text
generate new lockfile
validate
write temporary file
rename into place
```

An interrupted operation should not leave a partially written lockfile.

---

## 12. Lockfile Format Version

The lockfile includes an explicit schema version:

```toml
lock-version = 2
```

This field versions the lockfile representation, not the Vut package version.

Version 2 adds per-target native artifact pinning (`[[package.native]]`).
Version 1 lockfiles remain readable and are upgraded to version 2 on the next
successful resolution or pinned install.

---

## 13. Compatibility

Newer VPM versions should read older compatible lockfiles where practical.

Older VPM encountering an unsupported newer lockfile must fail clearly rather than silently ignoring critical information.

---

## 14. Manifest vs Lockfile

`vpm.toml` describes requested project dependencies.

`vpm.lock` records exact resolved dependency state.

Conceptually:

```text
vpm.toml
    ↓
resolver
    ↓
vpm.lock
```

---

## 15. Installation

When a valid lockfile exists:

```text
vpm install
```

uses the pinned resolution:

```text
no version re-resolution
no artifact re-resolution
no silent upgrade
```

Each package is fetched at its pinned exact version and verified against the
pinned revision and BLAKE3 checksum. Native artifacts are taken from the pinned
`[[package.native]]` URL/SHA-256 for the requested target. If a pinned URL is
unreachable and the artifact is not cached, install fails; it never searches for
a replacement artifact.

If the manifest requests a dependency that the lockfile does not pin, install
fails and asks the user to run `vpm update`. Only `vpm update` (or an explicit
add/remove resolution) rewrites the lockfile.

---

## 16. Update

`vpm update` may intentionally resolve newer allowed/current versions according to command semantics.

After successful resolution:

```text
vpm.toml
vpm.lock
```

must remain consistent.

---

## 17. Changed Remote Version

If a supposedly immutable package version no longer matches its locked revision/checksum, VPM must report an integrity error.

Do not silently accept modified source.

---

## 18. Missing Lockfile

If the project has dependencies but no lockfile:

```text
vpm install  -> error: run `vpm update`
```

`vpm update`, `vpm add`, and `vpm remove` resolve from the manifest and create
or rewrite the lockfile. A project with no dependencies writes an empty
`lock-version = 2` lockfile.

---

## 19. Invalid Lockfile

Malformed or inconsistent lockfiles must produce actionable errors.

Do not panic.

---

## 20. Rules

1. Lockfiles are machine-managed.
2. Lockfiles contain concrete versions.
3. Source identity is preserved.
4. Revision/checksum verify remote immutability.
5. Native artifacts are pinned per target with a trusted SHA-256.
6. Output is deterministic.
7. Writes are atomic.
8. Lockfile format is versioned (`lock-version = 2`).
9. Normal install/build use the pinned resolution and never re-resolve.
10. Only `update`/explicit resolution rewrites the lockfile.
11. A dead pinned URL with a cold cache is an error, never a silent substitute.
12. Remote mutation is an error.
13. Lockfile corruption must never alter dependency semantics silently.
