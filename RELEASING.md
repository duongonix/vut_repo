# Releasing Vut

This procedure applies to the private `vut_repo` (source of truth). Artifacts
are published to the public `duongonix/vut` repository.

## One-time setup

1. Create a fine-grained PAT (or GitHub App) with `contents: write` on
   **only** `duongonix/vut`.
2. Add it to `vut_repo` as the secret `VUT_RELEASE_TOKEN`.
3. Enable release immutability in `duongonix/vut`.

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
   * builds the six Tier-1 distributions;
   * assembles and verifies each artifact with `vut-dist`;
   * publishes one release with all archives and `SHA256SUMS` to
     `duongonix/vut`.
3. If the workflow fails before publishing, fix and push a new commit; the
   workflow can be re-run with `workflow_dispatch` for the tag.

## After release

1. Verify the release assets on `duongonix/vut` (archives + `SHA256SUMS`).
2. Smoke-test the installer on each OS:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/duongonix/vut/main/install.sh | sh
   ```

3. Announce. Do not modify a published release; cut a new version instead.

## Rollback

A published release is immutable. To roll back, publish the previous version
again as a new tag, or direct users to
`install.sh --version=<previous>`.
