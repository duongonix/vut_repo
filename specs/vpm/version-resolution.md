# VPM Version Resolution

## 1. Purpose

This document defines how VPM selects package versions.

Remote versions are represented as package subdirectories:

```text
v<semver>
```

---

## 2. Exact Version

Command:

```text
vpm add math@1.2.0
```

must resolve exactly:

```text
math/v1.2.0
```

If it does not exist:

```text
error
```

Do not silently choose another version.

---

## 3. Latest

These are equivalent:

```text
vpm add math
vpm add math@latest
```

Both request the latest stable package version.

---

## 4. Discovery

Given:

```text
math/
├── v0.9.0/
├── v0.10.0/
├── v1.0.0/
├── README.md
├── development/
└── test/
```

only:

```text
v0.9.0
v0.10.0
v1.0.0
```

are version candidates.

---

## 5. Semantic Comparison

Version sorting MUST use SemVer.

Do not compare version names lexicographically.

Example:

```text
v0.9.0
v0.10.0
```

Semantically:

```text
0.10.0 > 0.9.0
```

Use:

```text
semver
```

crate.

---

## 6. Stable Latest

Given:

```text
v1.2.0
v1.3.0
v2.0.0-alpha.1
```

stable latest is:

```text
v1.3.0
```

Prerelease versions are excluded from normal `latest`.

---

## 7. Explicit Prerelease

Explicit prerelease is allowed.

Example:

```text
vpm add math@2.0.0-alpha.1
```

If:

```text
v2.0.0-alpha.1/
```

exists and is valid, it may be installed.

---

## 8. Invalid Version Directories

Ignore malformed version directories during version discovery.

Examples:

```text
v1
v1.2
version1.2.0
latest
foo
vabc
```

These do not participate in SemVer selection.

A malformed folder must not crash VPM.

---

## 9. Empty Package

If a package directory contains no valid version:

```text
error: package has no valid versions
```

The error should identify the package/source.

---

## 10. Manifest Verification

Selecting:

```text
v1.2.0
```

does not finish validation.

The manifest inside must also declare:

```toml
version = "1.2.0"
```

and the correct package name.

---

## 11. Concrete Manifest Write

If:

```text
vpm add math
```

selects:

```text
1.4.3
```

write:

```toml
[dependencies]
math = "1.4.3"
```

not:

```toml
math = "latest"
```

---

## 12. Lockfile

Lockfile also stores:

```text
1.4.3
```

not `latest`.

---

## 13. `@next`

`@next` is not currently part of VPM.

Do not implement it until explicitly specified.

---

## 14. Version Ranges

Syntax such as:

```text
^1.2.0
~1.2
>=1.0
*
```

is not automatically part of VPM.

Do not implement Cargo/npm-style version ranges until dependency semantics are formally specified.

---

## 15. Caching

Provider version listings may be cached.

However, explicit user operations that require fresh remote knowledge, such as checking for updates, must have well-defined cache-refresh behavior.

---

## 16. Determinism

Given the same set of remote version candidates:

```text
latest
```

must always resolve to the same version.

---

## 17. Rules

1. Exact versions resolve exactly.
2. Missing exact versions are errors.
3. No version means `latest`.
4. `@latest` means latest stable.
5. Prereleases are excluded from stable latest.
6. Explicit prerelease versions are allowed.
7. Malformed version folders are ignored.
8. SemVer comparison uses a mature library.
9. Resolved versions are persisted concretely.
10. Version ranges are deferred.
11. `@next` is deferred.
