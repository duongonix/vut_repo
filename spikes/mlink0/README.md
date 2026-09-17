# M-LINK.0 spike

Standalone native linking spike for Vut. Verifies that a compiled Vut object can
be linked into a native executable with the platform linker only, with **no
`rustc`/`cargo` in the link step**, keeping the runtime static.

See `specs/deploy/native-linking.md` for the full specification and measured
results.

## Layout

```text
fixtures/            Vut programs (each a module root with main.vut)
  fx-core-only/      pure core; observes via exit code (no out/print, no stdlib)
  fx-async/          async/await + vut(...) core runtime
  fx-stdlib/         fs/os/env native stdlib
  fx-http/           reqwest + tokio + rustls HTTP/TLS stack
  fx-payload-enum/   payload enums; excluded from the gate (pre-existing bug)
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

## CI results (M-LINK.0)

| Platform | Runner | Backend | Result |
| --- | --- | --- | --- |
| Windows x86_64 | windows-latest | link.exe / lld-link / cl.exe | PASS 4/4 |
| Linux x86_64 | ubuntu-latest | cc | PASS 4/4 (default PIE) |
| macOS arm64 | macos-latest | cc | PASS 4/4 (default PIE) |
| macOS x86_64 | macos-15-intel | cc | PASS 4/4 (default PIE) |

Codegen now emits position-independent objects (`is_pic`), so the default PIE
link works everywhere with no `-no-pie` workaround.

`fixtures/fx-payload-enum` is retained but excluded from the gate: it links on
every platform yet aborts at teardown on Linux/macOS. This is a pre-existing
compiler/runtime bug (it reproduces with non-PIC objects too), recorded in
`specs/deploy/native-linking.md` §12.

Downloaded evidence: `spikes/mlink0/report-ci5/` (git-ignored). See
`specs/deploy/native-linking.md` for the full analysis.
## Notes

* The runtime archives embed the Rust standard library. Linking both
  `vut-core` and `vut-stdlib` relies on archive extraction order and currently
  produces no duplicate-symbol errors on Windows; see
  `specs/deploy/native-linking.md` §4.
* `out/` and `report/` are git-ignored; committed assets are fixtures, tooling
  and scripts only.
