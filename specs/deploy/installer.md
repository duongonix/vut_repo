# Vut Installer

Status: implemented (M16–M19; manifest-driven resolution, mandatory integrity and
opt-in SDK provisioning added by the Production Release milestone). Sources:
`install/install.sh` (Linux/macOS) and `install/install.ps1` (Windows). They are
synced to the root of the public `duongonix/vut` repository and served from
`https://raw.githubusercontent.com/duongonix/vut/main/`.

## Requirements

Installing Vut requires only the tools already present on a normal system:

```text
Linux/macOS:  curl, tar, sha256sum or shasum
Windows:      PowerShell 5.1+
```

No Rust, Cargo, C compiler, Git or Node is required. (Compiling Vut programs
later requires the platform SDK/toolchain described in
[native-linking.md](native-linking.md).)

## Flow

```text
detect OS + architecture (+ glibc/musl on Linux)
  -> resolve the release manifest (releases.json)
       version-specific: <release>/releases.json
       latest:           duongonix/vut@main/releases.json
  -> select the target entry (explicit error for an unsupported OS/arch/libc)
  -> download the artifact from the manifest `url`
  -> verify SHA-256 against the manifest (MANDATORY; abort on mismatch)
  -> extract to a temporary directory
  -> verify manifest.json (target) and expected files
  -> atomic install into $VUT_HOME (bin/lib/std/manifest.json/version.json)
  -> place a bundled linker (`linker/`) on $VUT_HOME/bin when shipped
  -> configure PATH (unless --no-path)
  -> vut --version && vpm --version
  -> vut doctor (compile + link + run smoke test)
  -> on failure, print the exact official SDK provisioning step
```

The installers never construct asset URLs; they resolve everything through
`releases.json`. `VUT_RELEASE_MANIFEST` overrides the manifest source with a URL
or a local path (used by CI for offline clean-machine tests); plain-path and
`file://` asset URLs are accepted for offline installs.

## Self-managing toolchain (M2.5.6)

Locked principle: **"dependency cannot be bundled ≠ user must manually configure
it"**. The user must never have to locate `link.exe`, edit `PATH`/`LIB`/`INCLUDE`,
run `vcvars`, install Rust, or research a platform SDK just to `vut build
hello.vut`.

Rules:

* Do **not** redistribute Windows/macOS/Linux SDK components when licensing or
  platform rules forbid it. Use official provisioning/discovery mechanisms
  (platform package managers, official installer components, documented
  discovery) instead.
* A Vut-managed `lld` (LLVM, permissively licensed) may be shipped in the
  archive's `linker/` directory; the installer places it on `$VUT_HOME/bin`. LLD
  is **not bundled in v1**: on Windows it still requires the MSVC/UCRT import
  libraries, so it does not remove the SDK dependency.
* SDK provisioning is **opt-in**: the installer detects a missing SDK and prints
  the exact official command. `--provision` / `-ProvisionSdk` runs it.
  - Windows: `winget install --id Microsoft.VisualStudio.2022.BuildTools ...`
  - macOS: `xcode-select --install` (interactive; not auto-run)
  - Linux: the distribution package manager command
* `vut doctor` runs a compile + link + run smoke test and reports `READY` or
  actionable issues.

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
downloads an artifact with the wrong ABI. All eight targets are built and
published.

## Install root

```text
$VUT_HOME (default ~/.vut; %USERPROFILE%\.vut on Windows)
├── manifest.json
├── version.json
├── bin/
├── lib/runtime/<target>/
├── std/
├── packages/
├── cache/
└── config/
```

## Update safety

* Distribution-managed entries (`bin/`, `lib/`, `std/`, `manifest.json`,
  `version.json`) are replaced atomically; a failure restores the previous
  installation.
* User-managed data (`packages/`, `config/`) is never touched.
* `cache/` may be invalidated when the version or ABI changes.
* `rm -rf ~/.vut` is never used during an update.

## PATH

The installer appends an idempotent marker block to `~/.profile`, `~/.bashrc`
and `~/.zshrc` (Unix) or prepends `%USERPROFILE%\.vut\bin` to the user `Path`
(Windows). `--no-path` / `-NoPath` skips this. After install the installer
reports whether the **current** shell already sees `$VUT_HOME/bin`, and prints
the exact `export PATH=...` line plus a "open a new shell" note when it does not.

## Options

```text
install.sh   --version=vX.Y.Z   --no-path   --provision   -h/--help
install.ps1  -Version vX.Y.Z    -NoPath     -ProvisionSdk
```

Environment overrides: `VUT_HOME`, `VUT_VERSION`, `VUT_REPO`,
`VUT_RELEASE_MANIFEST`.
