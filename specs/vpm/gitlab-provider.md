# VPM GitLab Provider

## 1. Purpose

The GitLab provider allows VPM to use packages stored inside GitLab repositories.

GitLab uses an explicit provider prefix.

---

## 2. Source Syntax

Example:

```text
gitlab:nam/abc/math
```

Version command:

```text
vpm add gitlab:nam/abc/math@1.2.0
```

---

## 3. Interpretation

```text
provider      gitlab
owner/group   nam
repository    abc
package path  math
package name  math
```

Deep path:

```text
gitlab:nam/abc/libs/math
```

---

## 4. Package Structure

Example repository:

```text
abc/
└── math/
    ├── v0.1.0/
    ├── v1.0.0/
    └── v1.2.0/
```

The same VPM package rules apply as GitHub.

---

## 5. Repository Root Is Not Package

Invalid:

```text
gitlab:nam/abc
```

Valid:

```text
gitlab:nam/abc/math
```

---

## 6. Shared Provider Abstraction

GitLab must implement the same VPM provider abstraction used by GitHub.

Do not duplicate dependency-resolution logic.

Conceptually:

```text
PackageProvider
├── GithubProvider
└── GitlabProvider
```

---

## 7. GitLab-Specific Logic

Only GitLab provider code handles:

```text
GitLab APIs
GitLab authentication
GitLab rate limits
GitLab download endpoints
GitLab repository identifiers
```

---

## 8. Version Discovery

List package-directory children and retain only valid:

```text
v<semver>
```

folders.

SemVer comparison belongs to shared VPM version-resolution code.

Do not implement GitLab-specific version sorting.

---

## 9. Manifest Validation

For:

```text
math/v1.2.0
```

require:

```toml
[package]
name = "math"
version = "1.2.0"
```

---

## 10. Download

Fetch only required package content where practical.

Do not require a complete Git clone for every installation.

---

## 11. Authentication

Credentials belong to VPM/global secure configuration mechanisms.

Never store secrets in:

```text
vpm.toml
vpm.lock
package source
```

---

## 12. Errors

Distinguish:

```text
repository/project not found
package path not found
version not found
authentication failure
permission denied
rate limit
network error
manifest mismatch
integrity failure
```

---

## 13. GitLab Deployment Variants

Initial support may target the primary GitLab service.

Support for arbitrary self-hosted GitLab instances must not be invented unless its source/configuration syntax is formally specified.

Architecture should nevertheless avoid unnecessarily coupling all GitLab logic to one hard-coded assumption.

---

## 14. Testing

Provider network behavior must be mockable.

Tests should verify provider-independent resolver behavior separately from GitLab API behavior.

---

## 15. Rules

1. GitLab requires `gitlab:` prefix.
2. Same package/version directory rules as GitHub.
3. Repository root is never a package.
4. SemVer logic is shared.
5. Resolver logic is shared.
6. GitLab API details remain provider-local.
7. Credentials never enter project files.
8. Remote manifest identity must be validated.
