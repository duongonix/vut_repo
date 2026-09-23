# Vut

Vut is a compiled, statically typed, native programming language with a small
runtime and a package manager. This repository (`duongonix/vut_repo`) is the
**source and development repository**: compiler, runtime, standard library,
tests, specs and release automation.

Prebuilt distributions, installers and end-user documentation live in the
public repository [`duongonix/vut`](https://github.com/duongonix/vut).

## Install (end users)

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/duongonix/vut/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/duongonix/vut/main/install.ps1 | iex
```

Then:

```sh
vut --version
vpm --version
vut doctor
```

No Rust, Cargo, C compiler, Git or Node is required. See
[`docs/installation.md`](docs/installation.md).

## Build from source

Requires a stable Rust toolchain.

```sh
cargo build --release -p vut-cli -p vpm -p vut-lsp
cargo run -p vut-cli -- run examples/math.vut
```

Run the full gate:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Repository layout

```text
crates/            compiler pipeline, runtime, linker, CLI, VPM, tooling
vut-stdlib/        standard library source and native runtime
specs/             language, compiler, stdlib and deployment specifications
docs/              end-user documentation (synced to duongonix/vut)
install/           install.sh / install.ps1 (synced to duongonix/vut)
benchmarks/        benchmark corpus and competitor sources
examples/          runnable example programs
```

The compiler pipeline is `source → lexer → parser → resolver → types → HIR →
MIR → optimizer → Cranelift → object → native linker`.

## Release

See [`RELEASING.md`](RELEASING.md) and [`specs/deploy/`](specs/deploy/README.md).
Tag `vX.Y.Z` from this repository; the `Release` workflow builds every supported
target from source, publishes the artifacts to `duongonix/vut`, and syncs the
public distribution files.

## License

MIT. See [`LICENSE`](LICENSE). Third-party attributions are in
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).
