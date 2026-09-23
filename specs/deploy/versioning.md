# Vut Version Model

Status: implemented (M2). Distribution version is the root `Cargo.toml`
`[workspace.package] version`. `vpm` carries its own version.

## Version domains

| Component | Source | Example |
| --- | --- | --- |
| Vut distribution (`vut`, `vut-lsp`, stdlib, runtime) | root `Cargo.toml` `[workspace.package] version` | `0.1.0` |
| `vpm` | `crates/vpm/Cargo.toml` `version` (own literal) | `0.1.0` |
| Runtime ABI | `vut_runtime::abi::VERSION` (integer) | `10` |
| Manifest format | `vut_dist::manifest::FORMAT_VERSION` | `1` |

The stdlib and native runtime move with the distribution version; the runtime
ABI is tracked separately as an integer because generated code and the runtime
must agree on it exactly (`vut_codegen::verify_runtime_abi`).

## Tags and release validation

* Releases are tagged `vX.Y.Z` matching the distribution version.
* The `Release` workflow refuses to build when the tag does not equal
  `v<root Cargo.toml version>`.
* The `manifest.json` in every artifact records `vut`, `vpm`, `stdlib`,
  `runtime`, `abi_version`, `target`, `archive`, `channel`, `profile`,
  `cranelift`, `build_commit` and optionally `minimum_os`.
* The release-level `releases.json` records the version, compiler (version,
  Cranelift, profile), `runtime_abi`, `stdlib_version`, `manifest_format`, and a
  `targets[]` entry per target with its URL, SHA-256 and size. Installers resolve
  targets through it instead of constructing URLs.
* Each archive is attested with GitHub build provenance; the attestation bundles
  are published as release assets.

## Compatibility

* Published releases are immutable; a fix becomes a new version.
* Compiler caches are disposable and keyed by compiler version, target, build
  mode, source hash and runtime ABI (`vut-incremental`).
* Before 1.0 the language may still change; every change is intentional and
  documented in `specs/`.
