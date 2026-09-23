# Production Release & Distribution

Status: **implemented (R1–R6)**; completion is gated on a real GitHub Release in
`duongonix/vut` and a clean install of the published artifact on every supported
platform.

Goal: on a clean Windows/Linux/macOS machine, a user runs **one installer
command** and then `vut build` works — without installing Rust/Cargo, without
finding a linker, without editing `PATH`/`LIB`/`INCLUDE`, without running
`vcvars`, and without downloading the runtime/stdlib manually.

## Two-repository architecture

```text
duongonix/vut_repo   source of truth: compiler, runtime, stdlib, tests, specs, Actions
duongonix/vut        public distribution: README, LICENSE, installers, releases.json,
                     docs/, benchmarks/, examples/, GitHub Releases
```

All builds run from source in `vut_repo`. The public repository receives only
production artifacts and public distribution files — never compiler/runtime
source, internal specs, or build sources.

## Supported targets (8)

```text
x86_64-pc-windows-msvc        aarch64-pc-windows-msvc
x86_64-unknown-linux-gnu      aarch64-unknown-linux-gnu
x86_64-unknown-linux-musl     aarch64-unknown-linux-musl
x86_64-apple-darwin           aarch64-apple-darwin
```

## Locked decisions

1. **musl**: build both `x86_64` and `aarch64` musl targets.
2. **SDK provisioning**: detect + **opt-in** official provisioning.
3. **Public sync**: automatic from the release workflow.
4. **`releases.json`**: committed to `duongonix/vut@main`.
5. **Provenance**: SHA-256 + GitHub artifact attestations (bundles published as
   release assets so a private source repository stays verifiable).
6. **self update**: deferred; the installer performs upgrades.
7. **LLD**: not bundled in v1 (platform linker + SDK provisioning); the
   `linker/` staging hook remains.

## Release flow

```text
duongonix/vut_repo
  -> tag vX.Y.Z
  -> GitHub Actions build + test (8 targets)
  -> vut-dist assemble per target (manifest.json, version.json, LICENSE, notices,
     runtime, stdlib, startup, archive, SHA-256)
  -> vut-dist release-manifest -> releases.json
  -> attest-build-provenance (bundles)
  -> publish GitHub Release in duongonix/vut
  -> sync install.sh/install.ps1/releases.json/README/LICENSE/CHANGELOG/SECURITY/
     CONTRIBUTING/docs/benchmarks/examples to duongonix/vut@main
  -> clean-install verification of the published artifact per OS
```

## Phases

| Phase | Scope | Status |
| --- | --- | --- |
| R1 | Release manifest (`releases.json`) + installer resolution + mandatory SHA-256 + 8-target matrix | done |
| R2 | `[profile.release]` (strip/LTO/abort) + artifact completeness (`LICENSE`, notices, `version.json`, manifest fields) | done |
| R3 | `vut doctor` extension (versions, SDK, PATH, `--json`) | done |
| R4 | Installer hardening (musl, arch, opt-in SDK, PATH UX, offline mode) | done |
| R5 | Clean-machine CI + installer tests + release gates + attestations + public sync | done |
| R6 | Root docs + public distribution files + sync contents | done |

## Definition of Done

A fresh Windows/Linux/macOS environment, one installer command, `vut doctor`
green, `vut build`/`vut run` working, and stdlib math + native stdlib working,
with no Rust/Cargo and no manual linker/SDK/PATH configuration. Where a
proprietary SDK is legally required, the installer detects and provisions it via
the official mechanism (opt-in).

## Relevant specs and files

- `specs/deploy/{distribution,installer,release,versioning,native-linking}.md`
- `crates/vut-dist` (`assemble`, `release-manifest`)
- `install/install.sh`, `install/install.ps1`
- `crates/vut-cli/src/doctor/`
- `.github/workflows/{ci,release}.yml`
- `RELEASING.md`, `docs/`, `public/`
