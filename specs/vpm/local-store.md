# VPM Local Store

## 1. Purpose

VPM stores downloaded packages globally rather than copying dependencies into every Vut project.

Default conceptual root:

```text
~/.vpm/
```

---

## 2. Directory Structure

Canonical direction:

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

---

## 3. Package Store

Packages are source-aware.

```text
~/.vpm/packages/
├── registry/
│   └── math/
│       ├── 0.1.1/
│       └── 1.0.0/
│
├── github/
│   └── nam/
│       └── abc/
│           └── math/
│               └── 0.1.1/
│
└── gitlab/
    └── nam/
        └── abc/
            └── math/
                └── 1.2.0/
```

---

## 4. No `v` Prefix Locally

Remote:

```text
math/v1.2.0/
```

Local:

```text
math/1.2.0/
```

The local version directory does not require the `v` prefix.

---

## 5. Deep Package Paths

Remote:

```text
nam/abc/libs/math/v1.2.0
```

must preserve enough source identity locally to avoid collision.

Conceptual:

```text
~/.vpm/packages/github/nam/abc/libs/math/1.2.0/
```

---

## 6. Package Contents

Installed packages contain source.

Example:

```text
1.2.0/
├── vpm.toml
├── src/
│   ├── lib.vut
│   └── ...
├── README.md
└── LICENSE
```

There is no public `.vutlib`.

---

## 7. Store Is Managed by VPM

The package store is VPM-managed.

Users should treat installed package directories as read-only.

VPM should not rely on users modifying files inside the store.

---

## 8. Shared Packages

Multiple projects may use:

```text
math 1.2.0
```

from the same source.

VPM should store it once and reuse it.

---

## 9. Package Identity

A local package identity includes:

```text
provider/source
owner/repository when applicable
package path
package name
version
```

Do not identify packages using only:

```text
name + version
```

inside the global store.

---

## 10. Installation Strategy

Package installation should conceptually:

```text
resolve
↓
check local store
↓
use existing verified package
OR
download
↓
validate
↓
install atomically
```

---

## 11. Partial Installation

Never expose a partially downloaded package as a valid installed package.

Use temporary locations before moving validated contents into the final store.

---

## 12. Validation

Before finalizing installation validate:

```text
manifest exists
manifest name
manifest version
source identity
package layout
integrity/checksum
path safety
```

---

## 13. Concurrent VPM Processes

Architecture must account for two VPM processes attempting to install the same package simultaneously.

Do not allow concurrent writes to corrupt the package store.

Use atomic operations and locking where required.

Use a proven locking strategy/library rather than fragile custom lock-file assumptions.

---

## 14. Package Removal

Normal:

```text
vpm remove math
```

removes the dependency from the project.

It does not necessarily delete the globally cached package immediately.

Global garbage collection may be introduced separately.

---

## 15. Offline Use

If all locked dependencies are already available and verified locally, normal builds should not require network access.

---

## 16. Store Lookup Performance

Do not recursively scan the entire global package store for every dependency lookup.

Package identity should map deterministically to a local path.

---

## 17. Corruption

If an installed package fails integrity validation:

```text
do not compile it
```

VPM may remove/redownload it when safe, otherwise report a clear error.

---

## 18. Rules

1. Packages are globally shared.
2. Store is source-aware.
3. Packages contain source code.
4. No `.vutlib`.
5. Local version directories omit `v`.
6. Store contents are VPM-managed.
7. Partial installs must never become valid packages.
8. Concurrent installs must be safe.
9. Verified local packages should work offline.
10. Project removal does not automatically delete global package data.
