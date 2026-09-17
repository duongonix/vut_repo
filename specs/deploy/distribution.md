# Vut Distribution Format

Status: implemented (M4, M5, M15). Assembly is performed by the `vut-dist`
tool (`crates/vut-dist`); release artifacts are produced per target and
published as a single GitHub Release.

## Artifact layout

Every artifact contains exactly:

```text
manifest.json
bin/
  vut[.exe]
  vpm[.exe]
  vut-lsp[.exe]
lib/runtime/<target>/
  vut-core.lib    | libvut-core.a
  vut-stdlib.lib  | libvut-stdlib.a
  vut-startup.obj | vut-startup.o
std/
  <official stdlib .vut source>
```

The official `.vut` stdlib source ships inside the artifact because the compiler
needs it when a program imports stdlib modules. No compiler, Rust workspace or
private source is included.

## Naming

```text
Windows:  vut-v<version>-<target>.zip
Unix:     vut-v<version>-<target>.tar.gz
```

Examples:

```text
vut-v0.1.0-x86_64-pc-windows-msvc.zip
vut-v0.1.0-aarch64-pc-windows-msvc.zip
vut-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
vut-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
vut-v0.1.0-x86_64-apple-darwin.tar.gz
vut-v0.1.0-aarch64-apple-darwin.tar.gz
```

Archives contain their contents at the archive root (`manifest.json`, `bin/`,
`lib/`, `std/`) so installers can extract directly into `VUT_HOME`.

## Manifest

```json
{
  "format_version": 1,
  "vut": "0.1.0",
  "vpm": "0.1.0",
  "stdlib": "0.1.0",
  "runtime": "0.1.0",
  "abi_version": 10,
  "target": "x86_64-unknown-linux-gnu",
  "archive": "vut-v0.1.0-x86_64-unknown-linux-gnu.tar.gz",
  "build_commit": "<git sha>",
  "minimum_os": null
}
```

Fields: `format_version` and `abi_version` gate compatibility; `target` and
`archive` gate integrity; `build_commit` records provenance; `minimum_os` is
optional and only emitted when known. The archive checksum is not stored inside
the manifest (it would be circular); checksums are published in `SHA256SUMS`.

## Checksums

The release publishes one `SHA256SUMS` file listing every archive:

```text
<sha256>  vut-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
<sha256>  vut-v0.1.0-x86_64-pc-windows-msvc.zip
...
```

Installers verify the archive against `SHA256SUMS` before extracting.

## Tool

```text
vut-dist --target <triple> --out dist --bin-dir target/release \
         --vpm-version <vpm> --build-commit <sha>
```

The tool copies the built binaries, renames the runtime archives to the
distribution names, copies the stdlib source, writes the manifest, archives the
tree, and records a SHA-256 checksum.
