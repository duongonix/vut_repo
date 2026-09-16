# VPM Cache

## 1. Purpose

VPM uses caches to improve:

```text
dependency resolution
downloads
metadata lookup
build performance
offline behavior
```

Cache is an optimization.

Cache is never the source of truth.

---

## 2. Structure

Canonical direction:

```text
~/.vpm/cache/
├── downloads/
├── registry/
└── build/
```

Additional internal subdirectories may be added when responsibilities remain clear.

---

# 3. Download Cache

```text
downloads/
```

may store:

```text
downloaded archives
temporary provider snapshots
verified package payloads
```

This prevents unnecessary repeated downloads.

---

# 4. Registry Metadata Cache

```text
registry/
```

stores provider/registry metadata such as:

```text
known version listings
manifest metadata
provider response metadata
revision information
```

Do not confuse registry metadata cache with installed packages.

---

# 5. Build Cache

```text
build/
```

stores compiler/build artifacts that can safely be regenerated.

Potential contents:

```text
object files
dependency build outputs
compiler fingerprints
internal IR cache
```

Exact compiler cache layout belongs to compiler/incremental-build specifications.

---

# 6. Cache Identity

Cache keys must include all inputs relevant to correctness.

Depending on cache type:

```text
provider
repository
package path
version
revision
checksum
compiler version
target
build mode
source hash
relevant flags
```

Do not reuse cache entries when identity cannot be proven.

---

# 7. Cache Validation

Before using cached remote package data, verify required identity/integrity information.

A stale cache may reduce freshness.

It must never cause a different package to masquerade as the requested package.

---

# 8. Cache Invalidation

Caches must have explicit invalidation behavior.

Potential invalidation causes:

```text
package revision changed
checksum mismatch
compiler version changed
target changed
source changed
cache schema changed
```

---

# 9. Corrupted Cache

Corrupted cache should normally be:

```text
discarded
recreated
```

rather than causing permanent project failure.

If network is unavailable and required data cannot be recovered, report the actual problem clearly.

---

# 10. Offline Operation

When:

```text
vpm.lock
```

pins dependencies and verified package source exists locally, builds should not require registry metadata refresh.

---

# 11. Metadata Freshness

Operations differ in freshness needs.

Normal build/install with valid lock state:

```text
prefer local data
```

Operations such as:

```text
vpm outdated
vpm update
vpm search
```

may require fresher provider information.

---

# 12. No Unnecessary Network

Do not make network requests merely because VPM started.

Commands that can operate entirely locally should remain local where practical.

---

# 13. Atomic Cache Writes

Important cache metadata should be written safely.

Partial responses/files must not be treated as valid completed cache entries.

---

# 14. Concurrent Access

Multiple VPM processes may access caches.

Cache design must tolerate:

```text
simultaneous reads
simultaneous download attempts
interrupted writes
```

without corruption.

---

# 15. Cache Cleanup

`vpm clean` may remove appropriate project/build cache depending on command semantics.

Global cache garbage collection may be introduced separately.

Do not automatically delete globally installed package source merely because a project is cleaned.

---

# 16. Cache Performance

Cache lookup should be direct/keyed.

Do not recursively scan large cache trees for every operation.

---

# 17. Security

Never trust cached remote content solely because it exists.

Where integrity data is available, validate it.

Do not deserialize untrusted cache data using unsafe assumptions.

---

# 18. Rules

1. Cache is optional optimization.
2. Cache is not source of truth.
3. Cache identity must be explicit.
4. Corrupt entries should be recoverable.
5. Valid locked packages should support offline builds.
6. Normal local commands should avoid unnecessary network requests.
7. Writes must tolerate interruption.
8. Concurrent VPM processes must not corrupt cache.
9. Build cache is disposable.
10. Package store and cache are distinct concepts.
