
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

Conceptual format:

```toml
[[package]]
name = "math"
version = "1.3.2"
source = "registry:math"
revision = "63a9d..."
```

A checksum may also be recorded:

```toml
checksum = "..."
```

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
  "json 1.2.0",
  "core-utils 2.0.1"
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

The lockfile should include an explicit schema version.

Recommended initial direction:

```toml
lock-version = 1
```

This field versions the lockfile representation, not the Vut package version.

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

should prefer the locked resolution.

It must not query `latest` and unexpectedly upgrade dependencies.

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

If no lockfile exists, VPM resolves dependencies from the manifest and creates one when appropriate.

---

## 19. Invalid Lockfile

Malformed or inconsistent lockfiles must produce actionable errors.

Do not panic.

---

## 20. Rules

1. Lockfiles are machine-managed.
2. Lockfiles contain concrete versions.
3. Source identity is preserved.
4. Revision/checksum should verify remote immutability.
5. Output is deterministic.
6. Writes should be atomic.
7. Lockfile format is versioned.
8. Existing lock resolution is preferred for normal install/build.
9. Remote mutation is an error.
10. Lockfile corruption must never alter dependency semantics silently.
