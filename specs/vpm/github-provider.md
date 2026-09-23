# VPM GitHub Provider

## 1. Purpose

The GitHub provider allows VPM to discover and download Vut packages stored inside GitHub repositories.

GitHub is the default self-host provider.

---

## 2. Source Syntax

Example:

```text
nam/abc/math
```

Interpretation:

```text
owner       nam
repository  abc
package     math
```

Deep path:

```text
nam/abc/libs/math
```

---

## 3. Version Layout

Repository:

```text
abc/
└── math/
    ├── 0.1.0/
    ├── 0.2.0/
    └── 1.0.0/
```

Each version directory is a complete independent package.

---

## 4. Repository Root Is Not Package

This is invalid as a Vut self-host package:

```text
nam/abc
```

because no package subfolder exists.

Minimum:

```text
nam/abc/math
```

---

## 5. Provider Interface

The GitHub implementation must live behind a provider abstraction.

Conceptually:

```text
PackageProvider
```

with operations such as:

```text
list_versions(package)
fetch_manifest(package, version)
download(package, version)
resolve_revision(...)
```

Exact Rust trait API may evolve.

---

## 6. GitHub-Specific Logic

Only GitHub provider code should know about:

```text
GitHub API endpoints
GitHub authentication
GitHub repository metadata
GitHub rate limits
GitHub download mechanisms
```

The dependency resolver must not contain this logic.

---

## 7. HTTP

Use a mature HTTP client.

Recommended:

```text
reqwest
```

Do not implement HTTP/TLS manually.

---

## 8. URL Handling

Use:

```text
url
```

or equivalent robust URL handling.

Do not concatenate untrusted URL components carelessly.

---

## 9. Version Discovery

For:

```text
nam/abc/math
```

provider lists children under:

```text
math/
```

Candidate directories:

```text
0.1.0
0.2.0
1.0.0
README.md
dev
```

Only valid version-directory names participate in version resolution.

---

## 10. Manifest Fetch

Before installing a version, VPM must inspect:

```text
math/1.0.0/vpm.toml
```

and validate:

```text
name = math
version = 1.0.0
```

---

## 11. Download

Download only the required package version.

Do not clone the entire repository by default merely to obtain one package if GitHub APIs/archive mechanisms can retrieve the required content efficiently.

---

## 12. Revision

Provider should obtain a stable repository revision associated with downloaded content where practical.

Store it in the lockfile.

---

## 13. Authentication

Public packages should work without requiring authentication where GitHub allows it.

Authentication may later improve:

```text
rate limits
private repository access
```

Credentials must never be stored in project source or lockfiles.

---

## 14. Rate Limits

Provider must handle GitHub rate limits gracefully.

Do not repeatedly query the same package metadata when cached information is valid.

---

## 15. Errors

Provider errors should distinguish:

```text
repository not found
package path not found
version not found
permission denied
rate limited
network failure
invalid manifest
integrity failure
```

Do not collapse everything into:

```text
GitHub error
```

---

## 16. Security

Validate remote paths and downloaded content.

Never allow remote paths to escape VPM-controlled directories.

---

## 17. Testing

Provider implementation should support mocked HTTP responses.

Unit tests must not depend exclusively on live GitHub availability.

---

## 18. Rules

1. GitHub is the default self-host provider.
2. Source requires owner/repository/package.
3. Repository root is not a package.
4. Version folders use bare `<semver>` with no `v` prefix.
5. GitHub logic stays behind provider abstraction.
6. Download only necessary package content where practical.
7. Validate remote manifest identity.
8. Cache metadata responsibly.
9. Handle rate limits explicitly.
10. Never expose credentials in manifests/lockfiles.
