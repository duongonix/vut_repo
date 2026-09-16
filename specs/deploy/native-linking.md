# Vut Native Linking

Status: **M-LINK.0**. Measured on Windows (x86_64-pc-windows-msvc), Linux
(x86_64-unknown-linux-gnu) and macOS (aarch64-apple-darwin, arm64). macOS is
currently **blocked** by non-PIC code generation; see [§13](#13-gate).

This document is the source of truth for how a compiled Vut object is linked
into a native executable without `rustc`/`cargo`, and how the static runtime is
laid out in a distribution.

`rustc` is used in the M-LINK.0 spike **only to discover** the platform system
libraries (by capturing the arguments rustc passes to the linker). The final
link never invokes rustc or cargo.

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
* Artifacts are not merged with a script. The two archives stay separate.
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
linker. `rustc` is a development/migration-only fallback.

---

## 4. Internal build/link topology

| Distribution file | Internal crate | Internal lib |
| --- | --- | --- |
| `vut-core.lib` / `libvut-core.a` | `vut-runtime` | `vut_runtime` |
| `vut-stdlib.lib` / `libvut-stdlib.a` | `vut-stdlib-native` | `vut_stdlib_native` |

**Measured (Windows):** both archives embed the Rust standard library
(`std-*`, `core-*`, `alloc-*`, `panic_*`, `compiler_builtins`). Linking both
archives for the same executable produces **no** `LNK2005` duplicate
definitions, because the platform archive extractor pulls a member only while
it still resolves undefined symbols (`vut-core` is scanned first and defines
the std symbols).

**Measured (Linux):** `cc` with both `libvut-core.a` and `libvut-stdlib.a`
linked all four fixtures without duplicate-definition errors.

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

`LinkError` is classified so `vut doctor` can report:

```text
MissingSdk | MissingCrt | MissingLibrary | DuplicateSymbol | UnsupportedTarget
```

Requirements:

* No `env!("CARGO_MANIFEST_DIR")` on any production path.
* `--target` must be validated; when a target cannot be linked, fail with an
  explicit diagnostic instead of linking for the host.

---

## 6. Per-OS linking

### 6.1 Windows (MEASURED — PASS)

Link inputs (`x86_64-pc-windows-msvc`):

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
stage; `link.exe` is the smallest moving part. `lld-link` is retained as the
migration path toward the long-term bundled toolchain.

SDK requirement: **Windows SDK / Build Tools are required for the final link**.
Microsoft CRT/UCRT/SDK libraries are **not bundled** in MVP.

### 6.2 Linux (MEASURED — PASS)

`x86_64-unknown-linux-gnu`, `ubuntu-latest`, `cc` (GCC), `-no-pie`.

System libraries captured from rustc:

```text
-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
```

All four fixtures linked and ran with `vut-startup.o` + runtime archives only,
with no rustc/cargo in the link step.

Panic strategy note: `-lgcc_s` provides the unwinder for the debug/unwind
archive. Production `panic=abort` archives remove this requirement.

Self-contained Linux (bundled sysroot/CRT) is explicitly **post-MVP**.

Script: `spikes/mlink0/scripts/linux.sh`.

### 6.3 macOS (MEASURED — FAIL on arm64)

`aarch64-apple-darwin`, `macos-latest`, `cc` (clang).

System libraries captured from rustc:

```text
-lSystem -lc -lm
```

Link failure:

```text
ld: warning: -no_pie is deprecated when targeting new OS versions
ld: warning: -no_pie ignored for arm64*
Illegal text-relocations:
  text-relocation in '_vut_fn_1'+0x160 (...) to '_vut_rt_list_release_v1'
  ...
ld: Found illegal text-relocations
```

Root cause: the Cranelift backend emits **non-position-independent** objects
(`cranelift_codegen`'s `is_pic` setting defaults to `false`; see
`crates/vut-codegen/src/cranelift/compile.rs:39-49`). macOS arm64 forbids text
relocations and ignores `-no_pie`, so the object cannot be linked at all.

This also affects the existing rustc-based link path, i.e. macOS native
executables are currently not produced by Vut regardless of linker backend.

Required fix (not performed in M-LINK.0 — it is a compiler/codegen change):
enable `is_pic` in the Cranelift ISA flags and re-verify all targets. This is a
prerequisite for macOS, not a linker-backend concern.

macOS x86_64: not measured. GitHub no longer provides an Intel macOS
hosted runner (`macos-13` stayed queued indefinitely). Intel macOS verification
requires a self-hosted runner or cross-compilation plus Rosetta.

Script: `spikes/mlink0/scripts/macos.sh`.

---

## 7. System library matrix

| Component | Windows (measured) | Linux (measured) | macOS (measured) |
| --- | --- | --- | --- |
| CRT | `msvcrt`, `vcruntime`, `ucrt`, `oldnames` | system `libc`/CRT objects | `libSystem` |
| Rust std base | `kernel32`, `ntdll`, `userenv`, `ws2_32`, `dbghelp` | `gcc_s`, `util`, `rt`, `pthread`, `m`, `dl`, `c` | `System`, `c`, `m` |
| `getrandom` (ring 0.2) | `advapi32` | syscall | `Security` (framework; inferred) |
| `getrandom` (aws-lc 0.4) | `bcrypt` | syscall | `Security` (framework; inferred) |
| Networking (tokio/mio) | `ws2_32` | `pthread` | `libSystem` |
| Threads | `kernel32` | `pthread` | `libSystem` |

The Unix capture derives this list from rustc's own linker arguments. Framework
requirements that only the stdlib/HTTP stack pulls in (macOS `Security`) are not
visible in the dependency-free capture and are added explicitly by the script.

---

## 8. Panic strategy

* Production/release runtime artifacts use `panic = "abort"`.
* Rust panics must never unwind across the C ABI boundary.
* Development/test builds may keep `panic = "unwind"`.

**Measured (Windows):**

* `vut-core` and `vut-stdlib` built with `-C panic=abort` contain `panic_abort`
  and not `panic_unwind`.
* All four fixtures link and run correctly against the abort archives.
* Stable Cargo cannot build the test harness with `panic=abort`
  (`building tests with panic=abort is not supported without
  -Zpanic_abort_tests`). The suite therefore runs under the dev profile
  (unwind) on stable; abort artifacts are verified end-to-end by the fixtures.
  A full suite run under abort needs nightly and is deferred to CI.

The release profile change is pending until the full suite plus the abort
fixture run pass across the supported targets.

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
6. Remove link_shim.rs and the CARGO_MANIFEST_DIR dependency from production
7. Update specs/std to the two-archive + startup model
```

M-LINK.1–M-LINK.6 remain blocked until M-LINK.0's cross-OS evidence exists.
macOS requires the codegen PIC fix first.

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

## 12. Spike evidence

Spike assets live in `spikes/mlink0/` and the evidence is downloaded to
`spikes/mlink0/report-ci2/` (git-ignored) from the `M-LINK.0 spike` workflow.

### Results by platform

| Platform | Runner | Backend | Fixtures | Result |
| --- | --- | --- | --- | --- |
| Windows x86_64 | windows-latest | link.exe / lld-link / cl.exe | 4/4 | **PASS** |
| Linux x86_64 | ubuntu-latest | cc (`-no-pie`) | 4/4 | **PASS** |
| macOS arm64 | macos-latest | cc | 0/4 | **FAIL** (non-PIC) |
| macOS x86_64 | — | — | — | not measured (no hosted Intel runner) |

### Fixture outcomes

| Fixture | Windows | Linux | macOS arm64 |
| --- | --- | --- | --- |
| `fx-core-only` | exit 46 | exit 46 | link fails |
| `fx-async` | `async=42 vutcon=42` | `async=42 vutcon=42` | link fails |
| `fx-stdlib` | `os=windows, fs=1, has_path=1` | `os=linux, fs=1, has_path=1` | link fails |
| `fx-http` | `status=200` | `status=200` | link fails |

`panic=abort` variants of all four fixtures link and run on Windows.

### Baseline note

The repository's existing `CI` workflow is red for an unrelated reason:
`crates/vpm/src/testing.rs:123`
(`runner_filters_captures_and_reports_deterministically`) fails on Linux and
macOS. This is independent of M-LINK.0.

---

## 13. Gate

M-LINK.0 is complete only when all of the following hold:

- [x] Four fixtures link and run on Windows with no rustc/cargo in the link
- [x] Windows backend comparison (`link.exe` / `lld-link` / `cl.exe`)
- [x] Windows system-library list measured
- [x] Windows `panic=abort` artifacts link and run
- [x] No duplicate definitions for the two archives (Windows, Linux)
- [x] Four fixtures link and run on Linux
- [ ] Four fixtures link and run on macOS (blocked: non-PIC Cranelift output)
- [x] Fixtures and tooling committed
- [ ] `specs/deploy/native-linking.md` finalized with a green macOS result

**T1 is not finalized.** Per the accepted rule, T1 requires Windows + Linux +
macOS to all demonstrate link + run. macOS arm64 fails because Cranelift emits
non-PIC objects; macOS x86_64 has no hosted runner. M-LINK.1 must not start
until this is resolved.
