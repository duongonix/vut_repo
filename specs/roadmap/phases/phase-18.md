# Phase 18 — Package Resolution and Providers

## Status

Complete

VPM now parses structured registry/GitHub/GitLab package identities, resolves
exact or latest-stable SemVer directories, ignores malformed candidates, and
supports explicit prereleases. The provider-independent resolver builds a
deterministic transitive graph, shares diamonds, and rejects cycles, namespace
collisions, version conflicts, missing versions, unsafe paths, and remote
manifest identity mismatches.

GitHub and GitLab use provider-local HTTP implementations. The default registry
maps to `github.com/duongonix/vpm`. Downloads contain source only and are
installed atomically into a locked, source-aware global store. BLAKE3 checksums,
provider revisions, deterministic lock entries, immutability verification, and
offline reuse of valid locked packages are implemented.

Provider HTTP is mockable; version, source, graph, provider, project, and store
tests cover the Phase 18 contract without requiring live services.

Installation now revalidates the resolved snapshot at the store trust boundary
before creating staging content: checksum, UTF-8 manifest, manifest name/version,
non-empty revision, safe relative paths, and at least one `src/**/*.vut` file are
mandatory. Invalid packages never become visible at their final store path.
