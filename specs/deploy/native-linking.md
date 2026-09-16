# Vut Native Linking

Status: **M-LINK.0**. Windows behavior is measured. Linux and macOS procedures
are specified and scripted but **not yet executed** (blocked on CI; see
[§12 Evidence](#12-spike-evidence)).

This document is the source of truth for how a compiled Vut object is linked
into a native executable without `rustc`/`cargo`, and how the static runtime is
laid out in a distribution.

---

## 1. Goals

Two goals must never be conflated.

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

M-LINK.0 only establishes the MVP. It must not change compiler or codegen
architecture to work around linker limitations.

---

## 2. Public distribution layout

Distribution ABI/layout is stable. Internal crate names may differ.

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
* Artifacts are not merged together with a script. The two archives stay
  separate (see [§4](#4-internal-buildlink-topology)).
* The naming above is the distribution contract; `vut-runtime` /
  `vut-stdlib-native` remain internal crate names.

---

## 3. Standalone linking model

```text
Vut source
  -> compiler / codegen
  -> native object (defines `vut_entry`)
  -> linker backend (per target)
       + vut-startup.o      (defines `main`)
       + vut-core.a/.lib
       + vut-stdlib.a/.lib  (only when the program imports stdlib)
       + platform system libraries
  -> executable
```

The only external process allowed during production linking is the platform
linker. `rustc` is a development/migration-only fallback and must never be
required by a release installation.

---

## 4. Internal build/link topology

The public layout exposes two runtime archives. Internally each is produced by a
Rust `staticlib` crate:

| Distribution file | Internal crate | Internal lib |
| --- | --- | --- |
| `vut-core.lib` / `libvut-core.a` | `vut-runtime` | `vut_runtime` |
| `vut-stdlib.lib` / `libvut-stdlib.a` | `vut-stdlib-native` | `vut_stdlib_native` |

**Measured (Windows, x86_64-pc-windows-msvc):** both archives embed the Rust
standard library (`std-*`, `core-*`, `alloc-*`, `panic_*`,
`compiler_builtins`). Linking both archives for the same executable nevertheless
succeeds with no `LNK2005` duplicate-definition errors, because the platform
archive extractor pulls a member only while it still resolves undefined symbols:
`vut-core` is scanned first and defines the Rust std symbols, so `vut-stdlib`'s
duplicate std members are never extracted.

```text
vut-core.a   (16.7 MB debug)  -> std + core + compiler_builtins + async/Vutcon
vut-stdlib.a (305.3 MB debug) -> std + core + core deps + ring + aws-lc-rs
                                 + reqwest + tokio + hyper + rustls + webpki
```

Risk: extraction-order deduplication is an emergent property of the linker, not
an explicit contract. M-LINK.1 must add a broader fixture set (larger stdlib
surface, generics, interfaces, async) and, if any duplicate definition appears,
choose a corrected internal topology (T2/T4 in the spike plan) without changing
the public layout. No script-based merge is permitted.

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

`LinkError` is classified so `vut doctor` can report:

```text
MissingSdk | MissingCrt | MissingLibrary | DuplicateSymbol | UnsupportedTarget
```

Requirements:

* No `env!("CARGO_MANIFEST_DIR")` on any production path. The startup object and
  all runtime archives come from the resolved `VUT_HOME` layout.
* `--target` must be validated; when a target cannot be linked, fail with an
  explicit diagnostic instead of linking for the host.

---

## 6. Per-OS linking

### 6.1 Windows (MEASURED)

Measured link inputs (`x86_64-pc-windows-msvc`, host `windows-latest`-equivalent):

```text
link.exe /NOLOGO /SUBSYSTEM:CONSOLE /OUT:<exe>
  vut-startup.obj
  <program>.obj
  vut-core.lib  [vut-stdlib.lib]
  /defaultlib:msvcrt /defaultlib:vcruntime /defaultlib:ucrt /defaultlib:oldnames
  kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib
  bcrypt.lib advapi32.lib          # only when getrandom/ring/aws-lc are present
```

Facts established by the spike:

| Fact | Value |
| --- | --- |
| CRT model | **dynamic** (`/defaultlib:msvcrt`), matching rustc's default for this target |
| rustc's own system libs | `kernel32.lib` (×3), `ntdll.lib`, `userenv.lib`, `ws2_32.lib`, `dbghelp.lib` |
| Extra libs required by the stdlib/HTTP archive | `bcrypt.lib` (`BCryptGenRandom`, aws-lc-rs), `advapi32.lib` (`SystemFunction036`, ring 0.2 getrandom) |
| Core-only program | requires only `vut-startup` + `vut-core` + CRT + `kernel32/ntdll/userenv/ws2_32/dbghelp` |
| Backends that work | `link.exe` (VS Build Tools + Windows SDK), `lld-link` (rust-lld), `cl.exe` |

Backend decision for **MVP**: `link.exe` is the primary backend. All three
candidates required the same Windows SDK libraries, so `lld-link` does not yet
remove the SDK dependency; `link.exe` is the smallest moving part.
`lld-link` is retained as the migration path toward the long-term bundled
toolchain (LLVM `lld` is redistributable under Apache-2.0 WITH LLVM-exception).

SDK requirement: **Windows SDK / Build Tools are required for the final link**.
Microsoft CRT/UCRT/SDK libraries are **not bundled** in MVP.

### 6.2 Linux (SPECIFIED, PENDING)

* Backend: platform `cc` (or `ld.lld` once validated).
* Non-PIE link is expected for Cranelift objects (`-no-pie`) unless codegen is
  confirmed PIC on this target.
* CRT objects and `libc` come from the system. `libgcc_s`/unwinder dependency is
  removed when `panic=abort` is used.
* Self-contained Linux (bundled sysroot/CRT) is explicitly **post-MVP**.
* Script: `spikes/mlink0/scripts/linux.sh`.

### 6.3 macOS (SPECIFIED, PENDING)

* Backend: `clang` (Xcode Command Line Tools). `ld64.lld` is a future option.
* Frameworks/system libs expected for the stdlib stack: `libSystem`, and
  `Security`/`CoreFoundation` for `getrandom`.
* Apple SDK `.tbd` files are **not** bundled; Xcode Command Line Tools are
  required for the final link.
* Script: `spikes/mlink0/scripts/macos.sh`.

---

## 7. System library matrix

| Component | Windows (measured) | Linux (expected) | macOS (expected) |
| --- | --- | --- | --- |
| CRT | `msvcrt`, `vcruntime`, `ucrt`, `oldnames` | system `libc`/CRT objects | `libSystem` |
| Rust std base | `kernel32`, `ntdll`, `userenv`, `ws2_32`, `dbghelp` | `pthread`, `dl`, `m`, `rt`/`util` (glibc) | `libSystem` |
| `getrandom` (ring 0.2) | `advapi32` | getrandom syscall | `Security` |
| `getrandom` (aws-lc 0.4) | `bcrypt` | getrandom syscall | `Security` |
| Networking (tokio/mio) | `ws2_32` | `pthread` | `libSystem` |
| Threads | `kernel32` | `pthread` | `libSystem` |

The Unix scripts derive this list directly from the arguments rustc passes to
the platform linker, so the matrix is verified rather than guessed once the CI
spike runs.

---

## 8. Panic strategy

* Production/release runtime artifacts use `panic = "abort"`.
* `rustc`/`cargo` are never invoked during production linking.
* Rust panics must never unwind across the C ABI boundary (`vut_rt_*` exports,
  `vut_entry`).
* Development/test builds may keep `panic = "unwind"`.

**Measured (Windows):**

* `vut-core` and `vut-stdlib` built with `-C panic=abort`; archives contain
  `panic_abort` and not `panic_unwind`.
* All four fixtures link and run correctly against the abort archives.
* Stable Cargo cannot build the test harness with `panic=abort`
  (`building tests with panic=abort is not supported without
  -Zpanic_abort_tests`). Therefore the suite is run under the dev profile
  (unwind) on stable; the abort artifacts are verified end-to-end by the
  fixtures. Running the full suite under abort requires nightly
  `-Zpanic_abort_tests` and is deferred to CI.

The release profile change is pending until CI confirms the full suite plus the
abort-fixture run on all Tier 1 targets.

---

## 9. `vut doctor`

A new diagnostic command must detect and explain:

```text
VUT_HOME resolution and std/ presence
lib/runtime/<target>/{vut-core,vut-stdlib,vut-startup} presence
required platform SDK/toolchain:
  Windows  -> Windows SDK / Build Tools (link.exe)
  macOS    -> Xcode Command Line Tools
  Linux    -> cc / native linker
manifest version and abi_version agreement
```

Missing prerequisites must produce an actionable message, never a raw linker
failure.

---

## 10. Migration off `rustc`

```text
1. Introduce LinkerBackend; default stays rustc (all tests green)
2. Add native `cc`/`cl` backend; verify on three OSes
3. Add `lld` backend; verify on three OSes          (long-term)
4. Flip the release default to the native backend
5. CI "clean environment" gate: compile + run a fixture with no rustc/cargo
   on PATH
6. Remove link_shim.rs and the CARGO_MANIFEST_DIR dependency from production
7. Update specs/std to the two-archive + startup model
```

M-LINK.1–M-LINK.6 remain blocked until M-LINK.0's cross-OS evidence exists.

---

## 11. Dependencies and sizes

* Unused direct dependencies `bytes`, `url`, and `rustls` (default features) in
  `vut-stdlib-native` should be removed. Removing `rustls`'s default
  `aws-lc-rs` feature eliminates the CMake/C-compiler build requirement and
  removes the `bcrypt` system dependency on Windows.
* Debug sizes measured: `vut-core` 16.7 MB, `vut-stdlib` 305.3 MB. Release
  sizes with LTO/`strip`/`panic=abort` and the dependency cleanup are required
  before release packaging.

---

## 12. Spike evidence

Spike assets live in `spikes/mlink0/` (fixtures, tooling, scripts) and evidence
under `spikes/mlink0/report/`.

### Windows results (measured, `x86_64-pc-windows-msvc`)

| Fixture | link.exe | lld-link | cl.exe | Runtime result |
| --- | --- | --- | --- | --- |
| `fx-core-only` | ok | ok | ok | exit 46 (expected) |
| `fx-async` | ok | ok | ok | `async=42 vutcon=42` |
| `fx-stdlib` | ok | ok | ok | `os=windows`, `fs=1`, `has_path=1` |
| `fx-http` | ok | ok | ok | `status=200` (live HTTPS request) |

`panic=abort` variants of `fx-core-only`, `fx-async`, `fx-stdlib`, `fx-http`
also link and run correctly.

### Linux / macOS results

**Pending.** Run the `M-LINK.0 spike` GitHub Actions workflow, or
`spikes/mlink0/scripts/linux.sh` / `macos.sh`, and attach the
`spikes/mlink0/report/<os>/` output.

---

## 13. Gate

M-LINK.0 is complete only when all of the following hold:

- [x] Four fixtures link and run on Windows with no rustc/cargo in the link
- [x] Windows backend comparison (`link.exe` / `lld-link` / `cl.exe`)
- [x] Windows system-library list measured
- [x] Windows `panic=abort` artifacts link and run
- [x] No `LNK2005` duplicate definitions for the two archives (Windows)
- [ ] Four fixtures link and run on Linux
- [ ] Four fixtures link and run on macOS
- [x] Fixtures and tooling committed
- [ ] `specs/deploy/native-linking.md` finalized with Linux/macOS evidence

M-LINK.1 must not start until the Linux and macOS boxes are checked.
