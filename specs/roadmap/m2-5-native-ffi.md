# M2.5 — Native/FFI & Package Production Hardening

M2.5 hardens the whole native boundary — Vut source → compiler → stdlib →
`extern "C"`/`@link_name` → native ABI → native runtime/package → linker → OS —
so Vut can rely on native code and native packages in production. It adds no
language syntax and is not an optimizer phase.

Status: **In Progress.** M2.5.1–M2.5.7 are implemented: ABI v1 spec + FFI
correctness + resource/error hardening + cross-platform linking + self-managing
toolchain + ABI versioning, and **M2.5.5 is complete** (sub-phases 5.5.1–5.5.11:
package model + `[[bin]]`, command surface, `<semver>` layout, lockfile v2
pinned installs, self-hosted identity/conflict, global CLI install/uninstall/exec,
native build metadata, resolved artifact download/verification/cache,
deterministic transitive native propagation, review-gated publishing, and
tests/CI/docs). M2.5.8 remains.

### Implemented (M2.5.1–M2.5.3)

- **M2.5.1** — `specs/ffi/06-native-abi-v1.md` (normative Public Native ABI v1 +
  Runtime Handle ABI); reconciled `specs/ffi/01..05`, `specs/std/03`, `specs/19`.
- **M2.5.2** — `bool` allowed across FFI (target C `_Bool`); `ptr[resource[T]]`
  rejected; `@repr(C)` on opaque data rejected; `resource[T]` fields in
  `@repr(C)` rejected (`E8014`); capturing closures rejected as FFI callbacks
  (`E8013`); fixed `from_raw_parts(null, 0)` UB in `string_from_utf8`,
  `bytes_from_list`, and `list_slice`; `external_signature_for` guarded against
  aggregate returns. Move-only element types (e.g. `channel[resource[T]]`) now
  get a null retain callback instead of a codegen error.
- **M2.5.3** — null-safety for `vut_rt_channel_send_v1`/`recv_v1` and
  `vut_rt_interface_data`/`set_vtable`/`vtable`; native HTTP client/runtime
  construction aborts instead of unwinding across the C ABI (`guard::or_abort`);
  resource lifecycle covered across move/early-return/`?`/result/loop/await/
  Vutcon/Channel.

## 0. Locked decisions

1. **Clean-machine toolchain.** Official Vut prefers a Vut-managed/bundled
   `lld-link`/`rust-lld` so a clean machine can build native executables without
   installing Visual Studio Build Tools or Rust. MSVC/Windows SDK discovery via
   `vswhere` is a **fallback** when present. Requiring the user to install a Rust
   toolchain is not acceptable. B1 is fixed outright: backend selection must not
   depend on `cfg!(debug_assertions)`; Debug, Release, and installed `vut.exe`
   use the same `LinkerResolver`/toolchain discovery architecture.
2. **`str`/`bytes`/`list[T]`.** Keep the current runtime-handle ABI, documented
   as the **Vut-internal Runtime Handle ABI** used between the compiler, stdlib,
   and the official runtime. It is **not** the public raw ABI for arbitrary
   third-party C. The Public Native ABI v1 and the Runtime Handle ABI are
   documented as distinct layers.
3. **`bool`/`int`/`float`.** Allow `bool` across Native ABI v1 using the **target
   C ABI representation and calling convention for C `_Bool`** (not hard-coded to
   1 byte). Continue to reject `int` and `float` (size and semantics are not
   explicit enough for stable FFI); use fixed-width `i8..i64`, `u8..u64`, `f32`,
   `f64`.
4. **Per-target native artifacts.** Support both target-aware manifest metadata
   and a `native/<target>/` directory convention. The manifest is the
   authoritative source telling the resolver/linker which artifact to use; the
   directory convention keeps package layout deterministic and cache-friendly.
   Transitive native-dependency propagation and lockfile determinism are required.
5. **Integrity.** Trusted **expected-checksum/integrity** verification is in
   scope for M2.5. Full cryptographic publisher/package signing, identity/trust
   chains, and registry signing infrastructure are deferred to M2.6+. A
   self-computed checksum after download is **not** sufficient to verify an
   artifact's origin.

## 1. Current state (audited)

- `extern "C"` / `extern "C" async fn` parsed, checked, lowered, and codegen'd
  with the target C calling convention; `@link_name` overrides the symbol.
- FFI-safe set (`crates/vut-types/src/checker/typing.rs:366-411`): fixed-width
  numerics, `ptr[T]`, `@repr(C)`/opaque data via `ptr[T]`, C function pointers,
  `str`, `bytes`, `resource[T]`, `list[T]` (recursive), `void` return. Rejects
  `int`/`float`/`bool`/`map`/`result`/`T?`/`array`/`enum`/`dyn`.
- Extern parameters are borrowed; extern returns are owned; managed temporaries
  are released after the call.
- `resource[T]` is a move-only opaque handle with exactly-once release and
  payload-carried error state (no global `LAST_ERROR`).
- The runtime exports `vut_rt_*` symbols; the ABI version is checked only against
  a compile-time constant.

## 2. What to keep

Full FFI pipeline and diagnostics (`E8001`, `E8004`–`E8007`); move-only
`resource[T]`; the deterministic resource-error-state pattern; static-library
linking with `[native]`, `LinkPlan`, startup object, per-OS system libraries, and
`vut doctor` asset checks; VPM project lifecycle, source-aware global store,
BLAKE3 content checksum, atomic install, and lockfile format; distribution
staging + `SHA256SUMS` + installer; the 3-OS CI matrix and Linux ASan/LSan.

## 3. Blockers and phase mapping

| ID | Severity | Gap | Phase |
|---|---|---|---|
| B1 | BLOCKER | Release `vut` links via `link.exe` selected by `cfg!(debug_assertions)` with PATH-only discovery and no `LIB`; Debug uses `rustc`. Root cause of Debug-works/Release-fails. | M2.5.4, M2.5.6 |
| B2 | HIGH | Runtime ABI version never verified against the linked archive; `manifest.json:abi_version` unused. | M2.5.7 |
| B3 | HIGH | Capturing closures accepted as `extern "C"` callbacks (tagged heap pointer as a function address). | M2.5.2 |
| B4 | HIGH | `resource[T]` fields allowed in `@repr(C)` data while drop tracking is disabled. | M2.5.2 |
| B5 | HIGH | `from_raw_parts(null, 0)` UB (empty string/bytes/slice). | M2.5.2 |
| B6 | HIGH | VPM per-target native artifacts unimplemented; documented shape unparseable; dependency native libs not propagated; host-only build. | M2.5.5 |
| B7 | MEDIUM | Lockfile not used to pin resolution; no download/registry cache; GitLab revision = HEAD; no `--target`. | M2.5.5 |
| B8 | MEDIUM | No library search-path mechanism (`/LIBPATH`/`-L`). | M2.5.4 |
| B9 | MEDIUM | Panics can cross `extern "C"` entry points. | M2.5.3 |
| B10 | MEDIUM | `unsafe impl Sync` for list/map without mutation locking; `env::set_var` races Tokio threads. | M2.5.3 |
| B11 | MEDIUM | Runtime-handle ABI undocumented and conflicting with `specs/ffi/*`. | M2.5.1 |
| B12 | LOW | `_v1` naming inconsistency; dead `ALLOC/REALLOC/FREE`; two error protocols; incomplete leak instrumentation. | M2.5.7, M2.5.3 |

## 4. Native ABI v1 (public) vs Runtime Handle ABI (internal)

The layers are kept explicitly separate:

```text
Vut language repr  ≠  MIR repr  ≠  Cranelift repr
                   ≠  Public Native ABI v1
                   ≠  Vut-internal Runtime Handle ABI
```

**Public Native ABI v1** (for third-party C and native packages):

- Scalars: `i8..i64`, `u8..u64`, `usize`/`isize`, `f32`/`f64`. `bool` uses the
  **target C ABI representation and calling convention for C `_Bool`** (defined
  by the target C ABI, not hard-coded to 1 byte).
- `ptr[T]` with target pointer width/alignment.
- `@repr(C)`/opaque `data` **by pointer only**.
- `extern "C" fn(...) -> ...` callbacks (non-capturing).
- `void` return.
- Not exposed in v1: `map`, `result`, `T?`, `array`, `enum`, by-value `data`,
  `dyn`/`interface`, `future`/`vutcon`/`channel`. Use `ptr[T]` + accessors.

If Vut ever needs a locked 1-byte boolean ABI, it must be defined as a Vut
`uint8` ABI with C-facing APIs using `uint8_t`, and must **not** be called
`_Bool`. Until then, `bool` follows C `_Bool` per target because the boundary is
`extern "C"`.

**Vut-internal Runtime Handle ABI** (compiler/stdlib/official runtime only):

- `str`, `bytes`, `list[T]` cross as one runtime-owned handle pointer; parameters
  borrowed, returns owned. Third-party C must not use this representation; the
  documented alternative is `ptr[u8] + usize` length.
- `resource[T]` is a move-only owned handle usable across both layers.

Full normative text lives in `specs/ffi/06-native-abi-v1.md`.

## 5. Ownership across FFI

| Case | Rule |
|---|---|
| Borrowed input | Managed handles and `ptr[T]`; valid for the call duration only. |
| Owned input | Deferred (no syntax in v1); native must not free Vut handles. |
| Owned return | Handles and `resource[T]` transfer to Vut; dropped deterministically. |
| Borrowed return | Deferred. |
| Resource handle | Move-only, single owner, released exactly once by Vut. |
| Callback capture | Must be non-capturing; capturing is rejected (B3). |

No allocator mismatch is possible inside one statically-linked image; the ABI
forbids native code freeing Vut-managed memory with a different allocator.

## 6. Native resources and errors

- Keep the resource-error-state pattern (`FileHandle`, `Count`, `ReplyHandle`,
  `Snapshot`): creation always returns a resource; Vut checks a status/error
  accessor; `Drop` is always safe; the native handle is released exactly once.
- No global mutable error slot.
- Native errors map to `result[T,E]`, `T?`, a resource error state, or a runtime
  trap, per the existing envelope `[status][payload]`. The legacy `Count`
  protocol is documented and migrated gradually.
- Native must not unwind across the C ABI boundary. Official runtime `extern "C"`
  entry points get a panic guard.

## 7. Linking and toolchain

- Backend selection is a single, explicit policy, identical for Debug and
  Release. Order: `VUT_LINKER` override → bundled `lld-link`/`rust-lld` → MSVC
  via `vswhere` (with `LIB`/`INCLUDE` provisioned) → documented diagnostic.
- Library search paths (`/LIBPATH`, `-L`) become first-class in `LinkPlan`.
- No hard-coded developer paths. `vut doctor` validates the driver, SDK, and
  assets, and emits repair hints.
- The installer bootstraps the bundled linker so a clean machine can
  `vut build hello.vut` with no manual setup.

## 8. VPM native packages

Two-layer model (locked in M2.5.5):

```text
author/source package          registry / trusted CI
  vpm.toml                       native-artifacts.toml
  native/build.toml              - per-target absolute URL
  native/src/                    - trusted expected SHA-256
                                 - size, system libraries
(source/build metadata)     ≠    (resolved artifact metadata)
```

Flow: `vpm publish` source → review/merge → trusted CI builds + uploads artifacts
and generates `native-artifacts.toml` → consumer `vpm add`/`install` resolves
(lockfile-pinned) source + per-target artifact → download absolute URL → verify
**trusted expected SHA-256** → cache → propagate transitively → local absolute
paths into `LinkPlan` → link.

- `native/build.toml` selects a structured backend (`cmake`/`cargo`/`make`/`cc`,
  `custom` as escape hatch) with per-target overrides.
- Registry packages publish native **source**; arbitrary prebuilt binaries are
  rejected. Local development may use `[native] libraries`.
- `native-artifacts.toml` is registry/CI-controlled and is excluded from the
  source-package BLAKE3 checksum.
- Transitive native dependencies are collected across the graph, de-duplicated,
  and made deterministic before entering `LinkPlan`. No user `--native-lib`.
- Target version directories are `<semver>` (no `v` prefix).
- Lockfile v2 pins source (name/version/source/revision/BLAKE3) and native
  (target/URL/SHA-256/size); `install` never re-resolves.
- Missing artifact for a target, corrupted download, checksum mismatch, dead URL
  with a cold cache, and offline cache are explicit errors.

## 9. ABI versioning and compatibility

- Runtime exports `vut_rt_abi_version_v1() -> u32`; generated code references it
  so a mismatch fails at link with a clear diagnostic.
- The installed `manifest.json:abi_version` is consumed by build and `vut doctor`.
- Symbol naming policy: `vut_rt_<module>_<op>_vN`; add `_v1` to the unversioned
  interface/http symbols; document the fs/io `_v2` policy.
- Dead `ALLOC`/`REALLOC`/`FREE` constants are removed or implemented.

## 10. Test strategy

Primitive FFI matrix (`Vut→C→native→Vut`) for fixed-width ints, floats, pointers,
and `bool`; ownership create/move/return/drop/error/early-return with leak
counters; resource lifecycle; static-library link+run on all supported OSes with
the system backend; VPM target-specific artifact; negatives (missing symbol,
wrong target, incompatible ABI, corrupted artifact, unsupported FFI type, missing
toolchain); ASan/LSan on FFI/resource tests.

## 11. CI matrix

Windows/Linux/macOS: compile, link, run, FFI, resource lifecycle, and native
package. Add a release/system-backend link job per OS; extend ASan/LSan to
FFI/resource tests. Shared-CI timing is not a hard gate.

## 12. Sub-phases

### M2.5.1 — Native/FFI audit + ABI v1 specification

- Goal: freeze Public Native ABI v1 and the Runtime Handle ABI on paper.
- Problem: handle ABI undocumented/conflicting (B11); `bool` decision pending.
- Files: `specs/ffi/01..05`, new `specs/ffi/06-native-abi-v1.md`,
  `specs/std/03-native-runtime.md`, this roadmap.
- Architecture: three-layer separation; ownership table; handle-ABI layer;
  versioning/naming policy.
- Tasks: write ABI v1; reconcile `specs/ffi/*` with the implemented handle ABI;
  lock the `bool` representation; document the resource-error protocol and the
  pointer-width rule.
- Tests: spec examples compile (`E8004`/`E8005` expectations).
- Cross-platform: per-target pointer-width rule.
- Risks: freezing a wrong representation → mitigate by marking the handle ABI
  Vut-internal.
- DoD: ABI v1 merged; `specs/ffi/*` consistent; no code change.
- Depends: none.
- Deferred: owned-input/borrowed-return syntax.

### M2.5.2 — FFI type/layout/ownership correctness

- Goal: eliminate FFI soundness holes and UB.
- Problem: B3, B4, B5, sret asymmetry, `ptr[resource[T]]`, `@repr(C)` on opaque,
  `bool` unsupported.
- Files: `vut-types/src/checker/{typing.rs,analyze.rs}`,
  `vut-mir/src/lowering/{layout.rs,builder.rs}`,
  `vut-runtime/src/abi/{string.rs,list.rs}`,
  `vut-codegen/src/cranelift/signatures.rs`.
- Architecture: type rules encode the ABI; layout stays target-derived.
- Tasks: reject capturing closures as FFI callbacks (new diagnostic); reject
  `resource[T]` fields in `@repr(C)`; fix null/zero-length UB; make
  `external_signature_for` sret-aware (guard); reject `ptr[resource[T]]`; reject
  `@repr(C)` on opaque; allow `bool`.
- Tests: negative type-check tests; runtime ownership tests returning
  `str`/`bytes`/`list` from native with leak counters; empty-string/list/slice.
- Cross-platform: layout tests per target.
- Risks: over-restricting stdlib → build the stdlib.
- DoD: holes closed with tests; stdlib builds.
- Depends: M2.5.1.

### M2.5.3 — Native resource & error hardening

- Goal: deterministic resource lifecycle and error mapping; no panics across FFI.
- Problem: B9, B10, two error protocols, unchecked null derefs.
- Files: `vut-runtime/src/{resource.rs,abi/panic.rs,abi/interface.rs,channel.rs}`,
  `vut-stdlib/native/vut-runtime/src/{abi.rs,resource.rs,fs.rs,count.rs,env.rs}`.
- Architecture: one documented error protocol; panic guard macro for native
  `extern "C"` bodies.
- Tasks: unify/document error handling; add panic guard; null-safety for
  channel/interface; review `Sync`/`env::set_var`; test resource across
  move/early-return/`?`/result/panic/Vutcon/Channel/task.
- Tests: lifecycle matrix with live counters; error-path tests.
- Cross-platform: all three OSes.
- Risks: protocol change breaks stdlib decoders → keep envelope, migrate `Count`.
- DoD: protocol documented and used; no panics cross; leak counters zero.
- Depends: M2.5.1, M2.5.2.

### M2.5.4 — Cross-platform native linking

- Goal: reliable static linking on Windows/Linux/macOS.
- Problem: B1 (backend selection), B8 (search paths).
- Files: `vut-linker/src/{backend.rs,backend/system.rs,system_libs.rs,plan.rs,process.rs}`,
  `vut-compiler/src/compiler/session.rs`, `vut-cli/src/cli.rs`.
- Architecture: one backend-selection policy; `LinkPlan` gains library search
  paths.
- Tasks: remove `cfg!(debug_assertions)` selection; add `/LIBPATH`/`-L`; make
  failures actionable.
- Tests: system-backend link+run on all OSes; argv tests.
- Cross-platform: Windows first.
- Risks: dev convenience → keep a discoverable fallback.
- DoD: system-backend link+run green on supported OSes.
- Depends: M2.5.1.

### M2.5.5 — VPM native artifacts/packages

- Goal: packages ship/consume target-specific native artifacts with a
  deterministic, pinned, verifiable flow.
- Problem: B6, B7.
- Files: `vpm/src/{manifest,resolver,project,store,lockfile,provider,source,
  version,compatibility,tooling}.rs`, `vut-resolver/src/loader.rs`, new
  `vpm/src/{native,artifact,global,publish}.rs`, `vut-paths`, `vut-dist`.
- DoD: canonical package layout; `<semver>` version dirs; lockfile v2 pins
  source + native with trusted SHA-256; install is pinned/offline-capable;
  transitive deterministic `LinkPlan`; global install/exec/uninstall with
  collision detection; publish (default + self-hosted + `--dry-run`, source
  only); tests + fmt + clippy green.

Sub-phases (each: run tests → fix regressions → `fmt` → `clippy` → workspace
tests → update this roadmap before the next phase):

1. **M2.5.5.1 — Package model + `src/mod.vut` + `[[bin]]`.** *(done)* Library
   entry is only `src/mod.vut`; CLI entries are `[[bin]]` with `src/`-relative
   paths; `src/lib.vut` special-case removed; `[native] build` vs `libraries`
   split; package validation requires a library or a bin and existing bin paths.
   Depends: none.
2. **M2.5.5.2 — Command semantics + CLI surface.** *(done)* `install [pkg]`,
   `uninstall`, `exec [--bin] [-- args]`, `publish [registry] [--dry-run]` parse
   and route; global/publish handlers are explicit placeholders until 5.5.6 and
   5.5.10. Depends: none.
3. **M2.5.5.3 — Registry/source abstraction + `<semver>` layout.** *(done)*
   Dropped the `v` version-directory prefix in code/specs/fixtures; `latest` =
   highest stable; default registry `duongonix/vpm`. Depends: 5.5.1.
4. **M2.5.5.4 — Deterministic lockfile v2.** *(done)* Lock-driven pinned
   install (exact version/revision/BLAKE3, no re-resolve); v1 readable/upgraded;
   only `update` rewrites; dead pinned source is an error. Depends: 5.5.3.
5. **M2.5.5.5 — Self-hosted registry + identity/conflict.** *(done)*
   Source-independent import identity; same-name conflict rejected; GitLab
   revision pinned to the version-directory commit (not `HEAD`). Depends: 5.5.3.
6. **M2.5.5.6 — Global CLI install/uninstall/exec.** *(done)* `~/.vut/bin` +
   `installs.toml`; `exec` selects by name → sole bin → error listing bins;
   global name-collision detection; per-bin staging so multiple `[[bin]]`s
   compile independently. Depends: 5.5.1, 5.5.3.
7. **M2.5.5.7 — Native source/build metadata.** *(done)* `[native] build` +
   structured `native/build.toml` (`cmake`/`cargo`/`make`/`cc`, `custom` escape
   hatch) with per-target overrides; registry packages reject prebuilt
   libraries/artifacts. Depends: 5.5.1.
8. **M2.5.5.8 — `native-artifacts.toml` + download + trusted SHA-256 + cache.**
   *(done)* Registry/CI metadata (excluded from the source BLAKE3 checksum);
   exact-target selection; absolute-URL download; trusted SHA-256 + size
   verification; content-addressed cache; per-target lockfile pinning.
   Depends: 5.5.7.
9. **M2.5.5.9 — Transitive native propagation + `LinkPlan`.** *(done)*
   Deterministic, de-duplicated `NativePlan` feeding
   `native_libraries`/`library_search_paths`/`system_libraries`; no user
   `--native-lib`. Depends: 5.5.8, 5.5.3.
10. **M2.5.5.10 — Publish workflow.** *(done)* Validate source → submit a
    review change/PR; source-only (rejects prebuilt binaries and publisher
    `native-artifacts.toml`); provider/auth abstracted (`VPM_TOKEN`/
    `GITHUB_TOKEN`); `--dry-run`. Depends: 5.5.1, 5.5.7.
11. **M2.5.5.11 — Tests/CI/docs.** *(done)* Unit + mock-HTTP integration +
    CLI-level E2E + negative matrix; 3-OS workspace CI; specs reconciled.
    Depends: all prior.

### M2.5.6 — Toolchain/linker discovery & installer integration

- Goal: official Vut installs and runs a native toolchain on a clean machine
  with **zero manual configuration**.
- Locked self-managing toolchain UX (target):

  ```text
  clean machine
  → detect OS/arch
  → install Vut
  → install/provision Vut-managed linker/toolchain
  → detect required platform SDK/system dependencies
  → reuse them if available
  → otherwise automatically provision missing dependencies through
    official/platform-supported mechanisms where possible
  → configure/discover required library/search paths
  → vut doctor
  → compile + link + run smoke test
  → READY
  ```

- Locked principle:

  ```text
  "dependency cannot be bundled ≠ user must manually configure it"
  ```

- Rules:
  - The user must never have to locate `link.exe`, edit `PATH`/`LIB`/`INCLUDE`,
    run `vcvars`, install Rust, or research/configure a platform SDK just to run
    `vut build hello.vut`.
  - Do **not** redistribute Windows/macOS/Linux SDK components when licensing or
    platform rules do not permit it. In that case use official
    provisioning/discovery mechanisms where possible (platform package managers,
    official installer components, documented discovery) instead of shipping them.
  - Debug, Release, and the installed `vut.exe` must all use the **same**
    `LinkerResolver`/toolchain discovery architecture — no `cfg!(debug_assertions)`
    divergence.
- Architecture:
  - New `LinkerResolver` in `vut-linker` is the single source of truth for
    toolchain discovery, shared by `vut build`/`vut run`/`vut doctor`/VPM and by
    Debug/Release.
  - Resolution order: `VUT_LINKER` override → Vut-managed/bundled
    `lld-link`/`rust-lld` → MSVC via `vswhere` with `LIB`/`INCLUDE` provisioning →
    documented, actionable diagnostic.
  - The installer provisions the Vut-managed linker and records it in the install
    manifest; the resolver locates it via `vut-paths`.
- Tasks:
  - Implement `LinkerResolver` and route all backends through it.
  - Provision the Vut-managed linker at install time; record it in
    `manifest.json`.
  - Detect platform SDK/system dependencies; reuse if present; otherwise
    auto-provision via official/platform mechanisms where permitted; otherwise
    emit a precise, actionable one-step diagnostic.
  - Derive library/search paths from the resolved toolchain/SDK into `LinkPlan`.
  - `vut doctor`: detect OS/arch, Vut-managed linker, platform SDK, system deps,
    library paths, manifest/ABI; run a compile+link+run smoke test; report
    `READY`/issues with repair hints.
  - Installer: detect OS/arch → install Vut → provision linker → run the doctor
    smoke test → only then report success.
- Tests: detection unit tests (mocked toolchains); doctor output tests; installer
  smoke test; a CI job that installs on a clean runner and runs
  `vut build hello.vut` with no manual setup.
- Cross-platform: Windows first, then macOS/Linux.
- Risks: licensing may forbid bundling some components → fall back to official
  provisioning/discovery (never require manual config); bundle size; CI time.
- DoD: on a clean machine (per supported OS) the locked UX sequence completes and
  `vut build hello.vut` produces a runnable executable with no manual
  toolchain/SDK/PATH work; Debug/Release/installed share one resolver.
- Depends: M2.5.4.
- Deferred: signed/managed toolchain updates, cross-target toolchains.

### M2.5.7 — ABI versioning + compatibility

- Goal: detect compiler/runtime/package ABI mismatch.
- Problem: B2, B11 (naming), B12 (dead alloc).
- Files: `vut-runtime/src/{abi.rs,lib.rs}`, `vut-codegen`,
  `vut-compiler/session.rs`, `vut-dist/src/{manifest.rs,stage.rs}`,
  `vut-cli/doctor.rs`.
- Architecture: runtime-exported ABI version referenced by generated code;
  manifest consumed.
- Tasks: export `vut_rt_abi_version_v1`; compiler reference; consume manifest;
  enforce `_vN`; remove/implement `ALLOC/REALLOC/FREE`; complete leak
  instrumentation.
- Tests: ABI mismatch fails with a clear error; naming lint.
- Cross-platform: all.
- Risks: ABI bump cadence → document policy.
- DoD: mismatch detected and diagnosed; naming consistent.
- Depends: M2.5.1, M2.5.4.

### M2.5.8 — Cross-platform production gate

- Goal: evidence the boundary is production-ready.
- Problem: CI never exercises the release/system backend; sanitizers limited.
- Files: `.github/workflows/{ci.yml,mlink35-e2e.yml}`, new FFI/resource/linking
  tests.
- Tasks: release/system-backend link+run job per OS; extend ASan/LSan to
  FFI/resource; negative matrix.
- Tests: full matrix.
- Cross-platform: three OSes.
- Risks: CI minutes → focused suite.
- DoD: matrix green; sanitizers green.
- Depends: all prior.

## 13. Definition of Done (whole M2.5)

FFI ABI documented (ABI v1 + Runtime Handle ABI) + ownership across FFI
deterministic + native resources safe + no Rust ABI leakage in the public native
ABI + native static linking reliable + VPM target-specific native artifact works
+ ABI incompatibility detected + Debug/Release linker behavior consistent +
clean-machine install/build/run works + cross-platform and memory-safety tests
green. A single working FFI demo is not sufficient.

## 14. Deferred to M2.6+

Full cryptographic publisher/package signing, identity/trust chains, registry
signing infrastructure; dynamic libraries; by-value `@repr(C)`;
`@repr(transparent)` ABI equivalence; version ranges; registry server;
`login/publish/yank`; 32-bit targets; arbitrary Rust/C++ ABI.

## 15. Files to change

- Specs: this file; `specs/ffi/06-native-abi-v1.md`; `specs/ffi/01..05`;
  `specs/std/03-native-runtime.md`; `specs/deploy/*`; `specs/roadmap/features.md`.
- Code (later phases): `crates/vut-linker/*`, `crates/vpm/*`,
  `crates/vut-types/src/checker/*`, `crates/vut-mir/src/lowering/*`,
  `crates/vut-runtime/src/*`, `vut-stdlib/native/vut-runtime/src/*`,
  `crates/vut-codegen/src/cranelift/signatures.rs`, `crates/vut-cli/src/doctor.rs`,
  `install/*`, `.github/workflows/*`.

## Verification

`cargo fmt --check`, `cargo clippy --workspace --all-targets`, and
`cargo test --workspace` are clean.
