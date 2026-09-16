# M-LINK.0 spike

Standalone native linking spike for Vut. Verifies that a compiled Vut object can
be linked into a native executable with the platform linker only, with **no
`rustc`/`cargo` in the link step**, keeping the runtime static.

See `specs/deploy/native-linking.md` for the full specification and measured
results.

## Layout

```text
fixtures/            four Vut programs (each a module root with main.vut)
  fx-core-only/      pure core; observes via exit code (no out/print, no stdlib)
  fx-async/          async/await + vut(...) core runtime
  fx-stdlib/         fs/os/env native stdlib
  fx-http/           reqwest + tokio + rustls HTTP/TLS stack
startup/             vut-startup.rs -> startup object exporting `main`
objgen/              standalone cargo project emitting the executable object
scripts/windows.ps1  3 linkers x 4 fixtures on Windows
scripts/unix.sh      shared Linux/macOS driver
scripts/linux.sh     Linux entry point
scripts/macos.sh     macOS entry point
out/                 build output (git-ignored)
report/              evidence/logs (git-ignored)
```

`vut-objgen` reproduces the compiler pipeline up to (but not including) linking
so the spike can drive a raw linker itself.

## Windows

```powershell
# from spikes/mlink0
pwsh -File scripts/windows.ps1
```

Requires Visual Studio Build Tools (C++ tools + Windows SDK) and a rustup
toolchain (for `rust-lld`, used standalone as `lld-link`).

Outputs:

```text
report/windows/spike.log        human-readable summary
report/windows/<fixture>-<backend>.log   linker output
report/windows/<fixture>-<backend>.stdout.txt
```

## Linux / macOS

```sh
# from spikes/mlink0
chmod +x scripts/*.sh
./scripts/linux.sh      # or ./scripts/macos.sh
```

The Unix driver derives the system library list from the arguments rustc passes
to the platform linker, then raw-links the fixtures with `cc`.

CI: run the `M-LINK.0 spike` workflow (`.github/workflows/mlink0-spike.yml`),
which executes all three OSes and uploads each `report/<os>/` directory.

## Expected results

```text
fx-core-only  exit 46
fx-async      stdout "async=42 vutcon=42", exit 0
fx-stdlib     stdout contains os=..., fs=..., has_path=..., exit 0
fx-http       stdout "status=200" (online) or a graceful "error=..." (offline)
```

## Notes

* The runtime archives embed the Rust standard library. Linking both
  `vut-core` and `vut-stdlib` relies on archive extraction order and currently
  produces no duplicate-symbol errors on Windows; see
  `specs/deploy/native-linking.md` §4.
* `out/` and `report/` are git-ignored; committed assets are fixtures, tooling
  and scripts only.
