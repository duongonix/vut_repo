# Vut Installer

Status: implemented (M16–M19). Sources: `install/install.sh` (Linux/macOS) and
`install/install.ps1` (Windows). They are published to the public
`duongonix/vut` repository.

## Requirements

Installing Vut requires only the tools already present on a normal system:

```text
Linux/macOS:  curl, tar
Windows:      PowerShell 5.1+
```

No Rust, Cargo, C compiler, Git or Node is required. (Compiling Vut programs
later requires the platform SDK/toolchain described in
[native-linking.md](native-linking.md).)

## Flow

```text
detect OS + architecture (+ glibc/musl on Linux)
  -> resolve version (VUT_VERSION or the latest GitHub release)
  -> map to a target triple
  -> download vut-v<version>-<target>.(zip|tar.gz) and SHA256SUMS
  -> verify SHA-256
  -> extract to a temporary directory
  -> verify manifest, target and expected files
  -> atomic install into $VUT_HOME
  -> configure PATH (unless --no-path)
  -> vut --version
```

## Target mapping

```text
Windows + x86_64            -> x86_64-pc-windows-msvc
Windows + ARM64             -> aarch64-pc-windows-msvc
Linux  + x86_64 + glibc     -> x86_64-unknown-linux-gnu
Linux  + x86_64 + musl      -> x86_64-unknown-linux-musl
Linux  + aarch64 + glibc    -> aarch64-unknown-linux-gnu
Linux  + aarch64 + musl     -> aarch64-unknown-linux-musl
macOS  + Intel              -> x86_64-apple-darwin
macOS  + Apple Silicon      -> aarch64-apple-darwin
```

The Linux installer distinguishes glibc from musl (`ldd /bin/sh`) so it never
downloads an artifact with the wrong ABI.

## Install root

```text
$VUT_HOME (default ~/.vut; %USERPROFILE%\.vut on Windows)
├── manifest.json
├── bin/
├── lib/runtime/<target>/
├── std/
├── packages/
├── cache/
└── config/
```

## Update safety

* Distribution-managed entries (`bin/`, `lib/`, `std/`, `manifest.json`) are
  replaced atomically; a failure restores the previous installation.
* User-managed data (`packages/`, `config/`) is never touched.
* `cache/` may be invalidated when the version or ABI changes.
* `rm -rf ~/.vut` is never used during an update.

## PATH

The installer appends an idempotent marker block to `~/.profile`, `~/.bashrc`
and `~/.zshrc` (Unix) or prepends `%USERPROFILE%\.vut\bin` to the user `Path`
(Windows). `--no-path` / `-NoPath` skips this.

## Options

```text
install.sh   --version=vX.Y.Z   --no-path   -h/--help
install.ps1  -Version vX.Y.Z    -NoPath
```

Environment overrides: `VUT_HOME`, `VUT_VERSION`, `VUT_REPO`.
