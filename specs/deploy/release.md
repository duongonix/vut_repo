# Vut Release Pipeline

Status: implemented (M13, M14, M20; release manifest, provenance and public sync
added by the Production Release milestone). Workflow:
`.github/workflows/release.yml`.

## Repository model

```text
duongonix/vut_repo   source of truth: compiler, runtime, stdlib, tests, specs, Actions
duongonix/vut        public distribution: README, LICENSE, installers, releases.json,
                     docs/, benchmarks/, examples/, GitHub Releases
```

Every build runs from source in `vut_repo`. The public repository receives only
production artifacts and public distribution files; it never receives compiler or
runtime source, internal specs, or build sources.

## Trigger and validation

```text
tag vX.Y.Z (or workflow_dispatch with an existing tag)
  -> validate tag == root Cargo.toml version
  -> read the vpm version from crates/vpm/Cargo.toml
  -> read the Cranelift version from Cargo.lock
  -> build the matrix (8 targets)
  -> assemble one distribution per target with `vut-dist assemble`
  -> merge per-target manifests into `releases.json` (`vut-dist release-manifest`)
  -> attest each archive's build provenance (upload bundles)
  -> publish ONE release to duongonix/vut (archives + manifests + releases.json +
     SHA256SUMS + *.sigstore.json)
  -> sync install.sh/install.ps1/releases.json/README/LICENSE/CHANGELOG/SECURITY/
     CONTRIBUTING/docs/benchmarks/examples to duongonix/vut@main
  -> clean-install verification of the published artifact on every supported OS
```

A tag that does not equal `v<root version>` fails before any build.

## Build matrix

Each target is built on a native runner with `cargo build --release --target
<triple>`; the startup object is prebuilt with `rustc --target <triple>`. No
cross-compilation is required.

| Runner | Target |
| --- | --- |
| windows-latest | x86_64-pc-windows-msvc |
| windows-11-arm | aarch64-pc-windows-msvc |
| ubuntu-latest | x86_64-unknown-linux-gnu |
| ubuntu-24.04-arm | aarch64-unknown-linux-gnu |
| ubuntu-latest | x86_64-unknown-linux-musl |
| ubuntu-24.04-arm | aarch64-unknown-linux-musl |
| macos-15-intel | x86_64-apple-darwin |
| macos-latest | aarch64-apple-darwin |

Windows jobs use `ilammy/msvc-dev-cmd`; Linux/macOS install NASM/CMake; musl jobs
install `musl-tools`.

## Cross-repository publishing and sync

Releases live in the public `duongonix/vut` repository. The default
`GITHUB_TOKEN` cannot write to another repository, so publishing and syncing use
a dedicated token with the minimum scope:

* a fine-grained PAT (or GitHub App installation token) with `contents: write`
  on **only** `duongonix/vut`, stored as the `VUT_RELEASE_TOKEN` secret in
  `vut_repo`;
* it is never hardcoded and never printed.

`gh release create --repo duongonix/vut` uploads all archives, per-target
manifests, `releases.json`, `SHA256SUMS` and attestation bundles. The
`sync-public` job clones `duongonix/vut`, copies the public distribution files,
and pushes a commit to `main` (idempotent: no commit when nothing changed).

## Provenance

`actions/attest-build-provenance` attests each archive. Attestations are recorded
in `vut_repo`; because that repository may be private, the bundles are also
uploaded as release assets so users can verify without repository access:

```text
gh attestation verify <archive> --bundle <archive>.sigstore.json \
  --signer-workflow duongonix/vut_repo/.github/workflows/release.yml
```

## Immutability

Published releases are immutable. Re-running the workflow for an existing tag
must not overwrite assets; publish a new version instead and enable repository
release immutability in `duongonix/vut`.

## Verification before publish

Each build job verifies the staged distribution (`manifest.json`,
`version.json`, `LICENSE`, `bin/`, `lib/runtime/<target>/`, `std/`, and the
manifest target/vpm/cranelift fields) after `vut-dist` runs. The
`verify-install` job installs the published release on a clean runner per OS,
runs `vut doctor`, and compiles and runs hello, a `std/math` program and a native
stdlib program.

## Release gates

Nothing is published unless formatting, clippy, workspace tests, ABI tests,
installer syntax/behaviour tests, the package smoke tests, and the per-target
builds all pass. See `.github/workflows/ci.yml`.
