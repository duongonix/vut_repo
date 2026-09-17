# Vut Release Pipeline

Status: implemented (M13, M14, M20). Workflow: `.github/workflows/release.yml`.

## Trigger and validation

```text
tag vX.Y.Z (or workflow_dispatch with an existing tag)
  -> validate tag == root Cargo.toml version
  -> read the vpm version from crates/vpm/Cargo.toml
  -> build the matrix
  -> assemble one distribution per target with vut-dist
  -> verify each distribution layout
  -> upload artifacts
  -> download all artifacts, compute SHA256SUMS
  -> publish ONE release to duongonix/vut
```

A tag that does not equal `v<root version>` fails before any build.

## Build matrix

Each target is built on a native runner; no cross-compilation is required.

| Runner | Target |
| --- | --- |
| windows-latest | x86_64-pc-windows-msvc |
| windows-11-arm | aarch64-pc-windows-msvc |
| ubuntu-latest | x86_64-unknown-linux-gnu |
| ubuntu-24.04-arm | aarch64-unknown-linux-gnu |
| macos-15-intel | x86_64-apple-darwin |
| macos-latest | aarch64-apple-darwin |

Windows jobs use `ilammy/msvc-dev-cmd` so `link.exe` and the Windows SDK are on
`PATH`. Linux/macOS install NASM for the crypto dependencies.

## Cross-repository publishing

Releases live in the public `duongonix/vut` repository. The default
`GITHUB_TOKEN` cannot write to another repository, so publishing uses a
dedicated token with the minimum scope:

* a fine-grained PAT (or GitHub App installation token) with
  `contents: write` on **only** `duongonix/vut`, stored as the `VUT_RELEASE_TOKEN`
  secret in `vut_repo`;
* it is never hardcoded and never printed.

`gh release create --repo duongonix/vut` uploads all archives plus `SHA256SUMS`
as a single release.

## Immutability

Published releases are immutable. Re-running the workflow for an existing tag
must not overwrite assets; publish a new version instead and enable repository
release immutability in `duongonix/vut`.

## Verification before publish

Each build job verifies the staged distribution (`manifest.json`, `bin/`,
`lib/runtime/<target>/`, `std/`, manifest target/vpm fields) after `vut-dist`
runs. The M-LINK workflows additionally prove the runtime links without
`rustc`/`cargo`; see [native-linking.md](native-linking.md).

## Dry runs

`workflow_dispatch` with an existing tag rebuilds and re-publishes. To rehearse
without publishing, run only the `validate` and `build` jobs (or run `vut-dist`
locally as described in [distribution.md](distribution.md)).
