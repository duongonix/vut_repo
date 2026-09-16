# Phase 17 — VPM Core

## Status

Complete

`vpm` implements `new`, `init`, `add`, `remove`, `install`, `update`, `build`,
`run`, and `check`. It validates TOML manifests with Serde/TOML and versions
with SemVer, preserves manifest formatting during edits, emits deterministic
versioned lockfiles atomically, establishes package/download/build caches, and
delegates semantic checking and native builds to `vut-compiler`.

Phase 17 accepts exact dependency versions and cached packages. Network-backed
latest-version/provider resolution remains Phase 18 by design.

Verified by VPM lifecycle, validation, overwrite-safety, and compiler tests.

The stabilized compatibility boundary rejects unknown identity, dependency, or
build-affecting manifest fields. Lockfile schema version 1 has an explicit
machine-readable supported range and rejects newer/older schemas, duplicate
identities, invalid SemVer, missing revisions, malformed dependency identities,
unknown fields, and non-BLAKE3 checksum shapes with actionable errors.
