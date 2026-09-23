# Releasing Vut

Vut uses two repositories:

- **`duongonix/vut_repo`** (this repository) — source of truth: compiler,
  runtime, stdlib, tests, specs and GitHub Actions. All builds run here from
  source.
- **`duongonix/vut`** — public distribution repository: README, LICENSE,
  installers, `releases.json`, end-user `docs/`, `benchmarks/`, `examples/`, and
  GitHub Releases. It never receives compiler/runtime source or `specs/`.

## One-time setup

1. Create a fine-grained PAT (or GitHub App) with `contents: write` on
   **only** `duongonix/vut`.
2. Add it to `vut_repo` as the secret `VUT_RELEASE_TOKEN`.
3. Enable release immutability in `duongonix/vut`.
4. Ensure GitHub artifact attestations are enabled for `vut_repo`
   (`actions/attest-build-provenance` needs `id-token: write` and
   `attestations: write`, granted by the workflow).

## Prepare

1. Update the distribution version in the root `Cargo.toml`
   `[workspace.package] version`.
2. Update `crates/vpm/Cargo.toml` `version` if VPM changed.
3. Update `CHANGELOG.md` and any affected `specs/`.
4. Run the full local checks:

   ```text
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```

## Tag and release

1. Commit, then tag `vX.Y.Z` matching the root version and push the tag.
2. The `Release` workflow:
   * validates `tag == v<root version>`;
   * builds the eight supported targets from source;
   * assembles each artifact with `vut-dist assemble` (manifest, `version.json`,
     LICENSE, notices, runtime, stdlib, startup object, archive, checksum);
   * merges the per-target manifests into `releases.json` with
     `vut-dist release-manifest`;
   * attests each artifact's build provenance and uploads the bundles;
   * publishes one release (archives, `*.manifest.json`, `releases.json`,
     `SHA256SUMS`, `*.sigstore.json`) to `duongonix/vut`;
   * syncs the public distribution files (`install.sh`, `install.ps1`,
     `releases.json`, README, LICENSE, CHANGELOG, SECURITY, CONTRIBUTING, `docs/`,
     `benchmarks/`, `examples/`) to `duongonix/vut@main`;
   * verifies a clean install of the published artifact on every supported OS.
3. If the workflow fails before publishing, fix and push a new commit; re-run
   with `workflow_dispatch` for the tag.

## Supported targets

```text
x86_64-pc-windows-msvc        aarch64-pc-windows-msvc
x86_64-unknown-linux-gnu      aarch64-unknown-linux-gnu
x86_64-unknown-linux-musl     aarch64-unknown-linux-musl
x86_64-apple-darwin           aarch64-apple-darwin
```

## After release

1. Verify the release assets on `duongonix/vut` (archives, `releases.json`,
   `SHA256SUMS`, attestation bundles).
2. Smoke-test the installer on each OS:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/duongonix/vut/main/install.sh | sh
   ```

3. Announce. Do not modify a published release; cut a new version instead.

## Verifying provenance

```sh
gh attestation verify <archive> \
  --bundle <archive>.sigstore.json \
  --signer-workflow duongonix/vut_repo/.github/workflows/release.yml
```

## Rollback

A published release is immutable. To roll back, publish the previous version
again as a new tag, or direct users to
`install.sh --version=<previous>`.
