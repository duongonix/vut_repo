# Vut Native Linking

Status: **M-LINK.0 complete**. All four fixtures link and run on Windows
(x86_64-pc-windows-msvc), Linux (x86_64-unknown-linux-gnu), macOS arm64 and
macOS x86_64, with **no `rustc`/`cargo` in the link step** and without
`-no-pie`/`-Wl,-no_pie`.

This document is the source of truth for how a compiled Vut object is linked
into a native executable without `rustc`/`cargo`, and how the static runtime is
laid out in a distribution.

`rustc` appears in the M-LINK.0 spike **only to discover** the platform system
libraries (by capturing the arguments rustc passes to the linker). The final
link never invokes rustc or cargo.

---

## 1. Goals

### MVP

```text
No rustc
No Cargo
Platform native SDK/toolchain may be required for the final link
Runtime is static (no .dll/.so/.dylib)
Public runtime layout is stable: vut-core / vut-stdlib / vut-startup
```

### Long-term

```text
No rustc / no Cargo
No external cc/cl where technically and legally practical
Bundled/minimal standalone linker (+ CRT/sysroot where legal)
Same compiler/runtime architecture; only the linker backend changes
```

M-LINK.0 establishes the MVP and does not change compiler/codegen architecture
except where required for correctness (position-independent code, see [§6.3](#63-macos-measured--pass)).

---

## 2. Public distribution layout

Windows:

```text
lib/runtime/<target>/
├── vut-core.lib
├── vut-stdlib.lib
└── vut-startup.obj
```

Linux / macOS:

```text
lib/runtime/<target>/
├── libvut-core.a
├── libvut-stdlib.a
└── vut-startup.o
```

Responsibilities:

```text
vut-core     language/runtime primitives: memory/ownership, str/bytes/list/map,
             dyn/interface, panic, bounds, async/Vutcon, core ABI (vut_rt_*)
vut-stdlib   native implementation of the official stdlib: fs/io/os/env/time/
             process/http/...
vut-startup  platform C entry `main` forwarding to `vut_entry`
```

Rules:

* `vut build` / `vut run` link startup + core + stdlib + required system
  libraries automatically. Users never manage these artifacts.
* `.rlib` files are **internal/development only**. They are never shipped as
  public runtime artifacts.
* Artifacts are not merged with a script. The two archives stay separate.
* The naming above is the distribution contract; `vut-runtime` /
  `vut-stdlib-native` remain internal crate names.

---

## 3. Standalone linking model

```text
Vut source
  -> compiler / codegen (emits position-independent object, defines `vut_entry`)
  -> linker backend (per target)
       + vut-startup.o      (defines `main`)
       + vut-core.a/.lib
       + vut-stdlib.a/.lib  (only when the program imports stdlib)
       + platform system libraries
  -> executable
```

The only external process allowed during production linking is the platform
linker. `rustc` is a development/migration-only fallback.

---

## 4. Internal build/link topology

| Distribution file | Internal crate | Internal lib |
| --- | --- | --- |
| `vut-core.lib` / `libvut-core.a` | `vut-runtime` | `vut_runtime` |
| `vut-stdlib.lib` / `libvut-stdlib.a` | `vut-stdlib-native` | `vut_stdlib_native` |

**Measured:** both archives embed the Rust standard library (`std-*`, `core-*`,
`alloc-*`, `panic_*`, `compiler_builtins`). Linking both for the same executable
produces no duplicate-definition errors on Windows (`LNK2005`) or Linux
(`multiple definition`), because the archive extractor pulls a member only while
it still resolves undefined symbols; `vut-core` is scanned first and defines the
std symbols.

```text
vut-core.a   (16.7 MB debug)  -> std + core + compiler_builtins + async/Vutcon
vut-stdlib.a (305.3 MB debug) -> std + core + core deps + ring + aws-lc-rs
                                 + reqwest + tokio + hyper + rustls + webpki
```

Risk: extraction-order deduplication is an emergent linker property, not an
explicit contract. M-LINK.1 must add a broader fixture set and, if any duplicate
definition appears, correct the internal topology. No script-based merge.

---

## 5. Linker abstraction

`crates/vut-linker` gains a per-target backend abstraction:

```rust
pub trait LinkerBackend {
    fn name(&self) -> &'static str;
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError>;
}
```

Selection order must eventually be:

```text
VUT_LINKER override (lld | cc | rustc; development only)
  -> bundled lld, when present            (long-term)
  -> platform native linker                (MVP)
  -> rustc                                 (development fallback only)
```

`LinkError` is classified so `vut doctor` can report
`MissingSdk | MissingCrt | MissingLibrary | DuplicateSymbol | UnsupportedTarget`.

Requirements:

* No `env!("CARGO_MANIFEST_DIR")` on any production path.
* `--target` must be validated; when a target cannot be linked, fail with an
  explicit diagnostic instead of linking for the host.

---

## 6. Per-OS linking

### 6.1 Windows (MEASURED — PASS)

```text
link.exe /NOLOGO /SUBSYSTEM:CONSOLE /OUT:<exe>
  vut-startup.obj
  <program>.obj
  vut-core.lib  [vut-stdlib.lib]
  /defaultlib:msvcrt /defaultlib:vcruntime /defaultlib:ucrt /defaultlib:oldnames
  kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib
  bcrypt.lib advapi32.lib          # only when getrandom/ring/aws-lc are present
```

| Fact | Value |
| --- | --- |
| CRT model | **dynamic** (`/defaultlib:msvcrt`), matching rustc's default |
| rustc's own system libs | `kernel32.lib` (×3), `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, `dbghelp.lib` |
| Extra libs required by stdlib/HTTP | `bcrypt.lib` (`BCryptGenRandom`), `advapi32.lib` (`SystemFunction036`) |
| Backends validated | `link.exe`, `lld-link` (rust-lld), `cl.exe` |

Backend decision for **MVP**: `link.exe`. All three candidates required the same
Windows SDK libraries, so `lld-link` does not remove the SDK dependency at this
stage. `lld-link` remains the migration path toward the long-term bundled
toolchain.

SDK requirement: **Windows SDK / Build Tools are required for the final link**.
Microsoft CRT/UCRT/SDK libraries are **not bundled** in MVP.

### 6.2 Linux (MEASURED — PASS)

`x86_64-unknown-linux-gnu`, `ubuntu-latest`, `cc` (GCC), **default PIE link**.

System libraries captured from rustc:

```text
-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
```

### 6.3 macOS (MEASURED — PASS)

`macos-latest` → `aarch64-apple-darwin`; `macos-15-intel` → `x86_64-apple-darwin`.

System libraries captured from rustc:

```text
-lSystem -lc -lm
```

plus `-framework Security -framework CoreFoundation` for the stdlib/HTTP stack
(getrandom).

All four fixtures link with clang's **default PIE** link and run.

**Codegen fix (authorized, part of M-LINK.0).** macOS rejects text
relocations, and `-no_pie` is ignored on arm64. The Cranelift backend previously
emitted non-position-independent objects (`is_pic` defaults to `false`). The
fix enables position-independent code in
`crates/vut-codegen/src/cranelift/compile.rs`:

```rust
flag_builder.set("is_pic", "true")?;
```

This is a relocation-model correctness fix, not a target-specific link
workaround. It applies uniformly to all targets (Windows and Linux continue to
link, now via default PIE rather than `-no-pie`).

Regression guard: `crates/vut-codegen/src/cranelift/tests.rs::emits_position_independent_code`
parses the emitted object and fails if any executable section contains an
absolute relocation. It was verified to fail with `is_pic=false` and pass with
`is_pic=true`.

---

## 7. System library matrix

| Component | Windows (measured) | Linux (measured) | macOS (measured) |
| --- | --- | --- | --- |
| CRT | `msvcrt`, `vcruntime`, `ucrt`, `oldnames` | system `libc`/CRT objects | `libSystem` |
| Rust std base | `kernel32`, `ntdll`, `userenv`, `ws2_32`, `dbghelp` | `gcc_s`, `util`, `rt`, `pthread`, `m`, `dl`, `c` | `System`, `c`, `m` |
| `getrandom` (ring 0.2) | `advapi32` | syscall | `Security` (framework) |
| `getrandom` (aws-lc 0.4) | `bcrypt` | syscall | `Security` (framework) |
| Networking (tokio/mio) | `ws2_32` | `pthread` | `libSystem` |
| Threads | `kernel32` | `pthread` | `libSystem` |

The Unix capture derives this list from rustc's own linker arguments; framework
requirements only pulled in by the stdlib/HTTP stack are added explicitly.

---

## 8. Panic strategy

* Production/release runtime artifacts use `panic = "abort"`.
* Rust panics must never unwind across the C ABI boundary.
* Development/test builds may keep `panic = "unwind"`.

**Measured (Windows):** abort `vut-core`/`vut-stdlib` contain `panic_abort`, not
`panic_unwind`; all four fixtures link and run against them. Stable Cargo cannot
build the test harness with `panic=abort` (needs nightly `-Zpanic_abort_tests`),
so the suite runs under the dev profile (unwind) on stable and the abort
artifacts are verified end-to-end.

---

## 9. `vut doctor`

Must detect and explain: `VUT_HOME` resolution, `std/` presence,
`lib/runtime/<target>/{vut-core,vut-stdlib,vut-startup}` presence, the required
platform SDK/toolchain (Windows SDK / Xcode CLT / cc), and manifest/ABI
agreement. Missing prerequisites must produce an actionable message, never a raw
linker failure.

---

## 10. Migration off `rustc`

```text
1. Introduce LinkerBackend; default stays rustc
2. Add native cc/cl backend; verify on all targets
3. Add lld backend; verify on all targets          (long-term)
4. Flip the release default to the native backend
5. CI clean-environment gate: compile + run with no rustc/cargo on PATH
6. Remove link_shim.rs and the CARGO_MANIFEST_DIR dependency from production
7. Update specs/std to the two-archive + startup model
```

---

## 11. Dependencies and sizes

* Unused direct dependencies `bytes`, `url`, and `rustls` (default features) in
  `vut-stdlib-native` should be removed. Removing `rustls`'s default
  `aws-lc-rs` feature eliminates the CMake/C-compiler build requirement and the
  Windows `bcrypt` system dependency.
* Debug sizes measured: `vut-core` 16.7 MB, `vut-stdlib` 305.3 MB. Release
  sizes with LTO/`strip`/`panic=abort` and dependency cleanup are required
  before release packaging.

---

## 12. Known pre-existing issues (not M-LINK.0, not fixed)

These are recorded separately and must not be mistaken for linking regressions.

1. **Payload-enum teardown crash (Linux/macOS).**
   `crates/vut-compiler/src/compiler/tests.rs::payload_enum_program_compiles_and_executes`
   produces fully correct stdout (`12 / 12 / 0 / exact`) and then aborts:
   Linux exit 134 with `malloc(): unaligned tcache chunk detected`; macOS exit
   133. The standalone fixture `spikes/mlink0/fixtures/fx-payload-enum` reproduces
   it. Isolation: the crash reproduces identically with `is_pic=false` and a
   `-no-pie` link, so it is **not** caused by the PIC change or by standalone
   linking; it is a memory-ownership bug in generated enum teardown that was not
   previously exercised on Linux/macOS because baseline CI fail-fast stopped at
   the `vpm` test.
2. **`vpm` unit test failure.** `crates/vpm/src/testing.rs:123`
   (`runner_filters_captures_and_reports_deterministically`) fails on Linux and
   macOS. Unrelated to M-LINK.0.

Both are outside M-LINK.0 scope. Neither blocks the four required M-LINK.0
fixtures.

---

## 13. Spike evidence

Spike assets: `spikes/mlink0/`. Evidence is downloaded from the `M-LINK.0 spike`
workflow into `spikes/mlink0/report-ci5/` and `report-exp-*` (git-ignored).

Final runs (`workflow_dispatch` on `main`):

| Platform | Runner | Backend | Fixtures | Link variant | Result |
| --- | --- | --- | --- | --- | --- |
| Windows x86_64 | windows-latest | link.exe / lld-link / cl.exe | 4/4 | n/a | **PASS** |
| Linux x86_64 | ubuntu-latest | cc / GCC | 4/4 | default PIE | **PASS** |
| macOS arm64 | macos-latest | cc / clang | 4/4 | default PIE | **PASS** |
| macOS x86_64 | macos-15-intel | cc / clang | 4/4 | default PIE | **PASS** |

| Fixture | Windows | Linux | macOS arm64 | macOS x86_64 |
| --- | --- | --- | --- | --- |
| `fx-core-only` | exit 46 | exit 46 | exit 46 | exit 46 |
| `fx-async` | `async=42 vutcon=42` | `async=42 vutcon=42` | `async=42 vutcon=42` | `async=42 vutcon=42` |
| `fx-stdlib` | `os=windows, fs=1, has_path=1` | `os=linux, fs=1, has_path=1` | `os=macos, fs=1, has_path=1` | `os=macos, fs=1, has_path=1` |
| `fx-http` | `status=200` | `status=200` | `status=200` | `status=200` |

Regression:

* `cargo test -p vut-codegen` passes on all four runners (including
  `emits_position_independent_code`).
* `cargo test -p vut-compiler` passes on Windows; on Linux/macOS it fails only
  in the pre-existing payload-enum test above.

---

## 14. Gate

- [x] Four fixtures link and run on Windows with no rustc/cargo in the link
- [x] Windows backend comparison (`link.exe` / `lld-link` / `cl.exe`)
- [x] Windows system-library list measured
- [x] Windows `panic=abort` artifacts link and run
- [x] No duplicate definitions for the two archives (Windows, Linux)
- [x] Four fixtures link and run on Linux (default PIE)
- [x] Four fixtures link and run on macOS arm64 (default PIE)
- [x] Four fixtures link and run on macOS x86_64 (`macos-15-intel`, default PIE)
- [x] PIC regression test passes on all runners and is a hard gate
- [x] Topology/staticlib behavior and system libraries recorded
- [x] Fixtures and tooling committed

**M-LINK.0 DoD met.** T1 link/run is demonstrated on Windows, Linux and both
macOS architectures. M-LINK.1 is the next milestone and must not rely on
`rustc`/`cargo` in production.
