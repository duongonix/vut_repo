# M2.4 — Optimizer & Performance

M2.4 makes the Vut compiler emit measurably faster release code without changing
language semantics, ownership, Drop, traps, or memory safety.

Backend note: the Vut native backend is **Cranelift 0.135.2**, not LLVM. M2.4 is
**Cranelift-only**; it does not add or evaluate an LLVM backend (that, if ever
needed, is separate roadmap work). This plan is written in Cranelift terms.

## 0. Correctness blocker (fixed in M2.4.2)

The previous Release optimizer was **not production-safe**. `vut_mir::optimize`
had an incomplete side-effect model: `side_effecting` / `result` did not cover
`Spawn`, `AwaitFuture`, `StartFuture`, `CallIndirect`, `InterfaceCall`, or
`OptionalFromValue`, so `eliminate_dead` could delete a spawn or an await whose
handle was unused (for example a discarded `vut(...)`), silently changing
scheduling and suspension.

**Fixed.** The optimizer now uses a complete, centralized effect model
(`crates/vut-mir/src/optimize/effects.rs`): observable instructions — stores,
drops, RC operations, `Spawn`, `AwaitFuture`, polls, calls, iterator steps,
interface construction, variadic access, and trapping division/remainder — are
never removed. A regression test (`optimizer_e2e::discarded_vutcon_spawn_survives_optimization`)
proves a discarded `Spawn` survives optimization.

### Implemented so far (M2.4.2–M2.4.8)

- **M2.4.2** — pass framework (`crates/vut-mir/src/optimize/`), `OptimizationLevel`
  (`O0..O3`, `--release ≡ O2`, Debug ≡ O0), MIR verifier (`verify.rs`), complete
  effect model, constant folding/propagation (`const_prop.rs`), constant-branch
  folding + unreachable-block removal (`branch.rs`), copy propagation
  (`copy_prop.rs`), fixed-point DCE (`dce.rs`). The pipeline verifies before and
  after and **restores the input if the optimized MIR fails verification**, so a
  buggy pass degrades to no optimization instead of miscompiling.
- **M2.4.3** — retain/release pair cancellation (`rc.rs`, invalidated by
  `MakeUnique`), redundant-drop removal (`drop.rs`), drop-after-move (`move_prop.rs`).
- **M2.4.4** — value escape analysis (`escape.rs`), fresh-aggregate `CopyAggregate`
  elision (`copy_elision.rs`), dead-allocation removal (`sroa.rs`).
- **M2.4.5** — collection `MakeUnique` elision for fresh constructors
  (`collections.rs`); bounds-check elimination (`bounds.rs`): variadic access with
  constant index/length, array access with a static length and constant index, and
  **list access in a `counter < list.len()` loop** rewritten to unchecked builtins
  (`ListAtUnchecked` / `ArrayAtUnchecked` + `vut_rt_list_at_unchecked_v1`). The
  fact is established on the taken branch and propagated through the dominator
  tree; values are canonicalized to the value they derive from (the loop counter
  and receiver are read through locals). Non-provable accesses keep their check
  and still trap.
- **M2.4.6** — async state-machine optimization (`async_opt.rs`): redundant frame
  reloads of the same slot are deduplicated. Spill slots are one machine word
  (8 bytes) instead of 16.
- **M2.4.7** — optimization levels wired end to end: `OptimizationLevel` on
  `CompilerConfig`, `vut build -O0..-O3` (`--release ≡ -O2`, explicit `-O` wins),
  backend `opt_level` selected from the level, and the level included in the
  incremental cache key.
- **M2.4.8** — differential correctness suite (`crates/vut-compiler/tests/optimizer_e2e.rs`):
  every case runs at `O0` and the optimized level (and a case across all four
  levels) and requires identical stdout and exit code; covers control flow,
  collections, managed values, optionals, map, variadics, interface dispatch,
  channels/Vutcons, and async+result. It runs in CI through
  `cargo test --workspace`.

Also fixed a latent scanner bug: `InterfaceCall.callee` was omitted from
`values::used` / `for_each_operand_mut`, so DCE could delete an interface callee
(and async spilling could miss one). It is now tracked as an operand.

### Verification

`cargo test --workspace` (0 failures), `cargo clippy --workspace --all-targets`,
and `cargo fmt --check` are clean. Differential tests
(`crates/vut-compiler/tests/optimizer_e2e.rs`) run every case at `O0` and `O2`
and require identical stdout and exit code; pass-level unit tests live in
`crates/vut-mir/src/optimize/tests.rs`. The full performance regression suite is
a manual/local or nightly gate (see §8); shared-CI timing is not a hard gate.

## 1. Current State

Pipeline (verified in code):

```text
Vut source
→ Lexer → Parser/AST → Resolver → Type Checker (SemanticResult)
→ HIR
→ MIR lower: ownership/drop + monomorphization   (vut-mir/src/lowering/lower.rs)
→ future::lower: async/Vutcon state machines      (lowering/future/mod.rs)
→ [Release only] vut_mir::optimize                (optimize.rs)
→ Cranelift codegen  opt_level = none(Debug) | speed(Release)
→ object → system linker → executable
```

- Driver: `vut-compiler/src/compiler/session.rs` (`optimize` only under
  `BuildMode::Release`, at `:345` and `:474`).
- Backend settings: `crates/vut-codegen/src/cranelift/compile.rs:39-55` — only
  `is_pic=true`; `opt_level=speed` when optimizing. No target CPU/features.
- Build modes: `BuildMode::{Debug, Release}` (`vut-compiler/src/compiler.rs`).
  CLI exposes `vut build --release` only. vpm has a boolean `release`, no
  `[profile]`.

### Existing optimizations

| Layer | What exists | Where |
|---|---|---|
| MIR | Intra-block int/bool constant folding; constant-branch folding; intra-block dead-instruction elimination | `vut-mir/src/optimize.rs` |
| MIR | Ownership-aware lowering: Copy/Move/Retain/Release/Drop/MakeUnique; COW detach (M2.3) | `lowering/builder*.rs`, `vut-memory` |
| MIR | Generics monomorphized | `lowering/monomorph.rs` |
| MIR | Inline collection iteration (no per-element call) | `codegen/instruction.rs`, `lowering/builder/loops.rs` |
| Codegen | Cranelift `opt_level=speed` in Release; PIC | `cranelift/compile.rs` |
| Runtime | COW make-unique fast path at refcount 1 (M2.3) | `abi/list.rs`, `abi/map.rs`, `bytes.rs` |
| Runtime | M:N scheduler, deferred reclamation, blocking pool (M2.2) | `scheduler/` |
| Runtime | Refcounted closure env; Vutcon capture (M2.1) | `closure.rs`, `futures.rs` |
| Correctness | CI fmt/check/test/clippy/fuzz; ASan/LSan on runtime/memory | `.github/workflows/ci.yml` |

### Missing / weak

> The audit below is the pre-M2.4 snapshot. Items marked **(done)** were
> addressed in M2.4.2–M2.4.5; the rest remain planned.

- **(done)** MIR pass framework + verifier (`crates/vut-mir/src/optimize/`).
- **(done)** Complete DCE side-effect model (see §0).
- **(done)** Constant/branch/copy propagation and unreachable-block removal.
- **(done)** Retain/release, drop, and move-out optimization.
- **(done)** Value escape analysis (`vut-mir` `optimize/escape.rs`).
- **(done)** Variadic bounds-check elimination (constant index/length).
- **Deferred:** list/array unchecked fast paths; loop induction/range analysis.
- **Deferred:** copy elision / SROA beyond fresh aggregates and dead allocations.
- **Deferred:** async frame compaction (16-byte spills, 32-byte iterator regions,
  per-spawn allocation).
- **Deferred:** target-CPU/feature tuning; inlining strategy; `-O` CLI levels.
- **Deferred:** benchmark corpus/history (M2.4.1); only `benchmarks/prime_count.*`
  exists.

## 2. Baseline Measurements

Only prebuilt artifacts were run (no build), 5 runs, warmup included:

| Benchmark | Vut (release) | Rust (-O3) | ratio |
|---|---|---|---|
| `prime_count` (<400k trial division) | 92.9 ms | 92.6 ms | 1.00x |

Cranelift `opt_level=speed` already matches LLVM -O3 on tight scalar integer
loops. M2.4's wins are therefore in Vut-specific semantics (ownership/RC, bounds,
allocation, collections, async), not scalar arithmetic. The full corpus baseline
is produced in M2.4.1.

## 3. Bottlenecks

- **B1** Bounds check + runtime call per collection access (`vut_rt_list_at_v1`).
- **B2** Unoptimized RC/COW traffic (Retain/Release/MakeUnique/Copy).
- **B3** Release optimizer nearly inert + unsafe DCE model (§0).
- **B4** Debug build unoptimized (Cranelift default opt_level).
- **B5** No target-CPU features (generic baseline).
- **B6** `map[K,V]` = `HashMap<Vec<u8>, Vec<u8>>`, default hasher, per-entry allocs.
- **B7** String concat/interpolation allocates per operation.
- **B8** Async frame overhead (16B spills, 32B iterator regions, per-spawn alloc).
- **B9** Limited inlining (no function attributes/strategy).
- **B10** Aggregate copies via `copy_aggregate`; no elision.

## 4. Optimization Architecture

```text
Vut semantic layer (MIR): ownership, drop, RC/COW, escape, bounds, async/closure
                         frames, Optional/Result, devirtualization
Backend (Cranelift): register allocation, instruction selection, per-function
                     DCE, scalar folding, block layout
```

MIR pipeline (new module tree; no monolithic file):

```text
crates/vut-mir/src/optimize/
  mod.rs          # pipeline driver + OptimizationLevel + report
  verify.rs       # SSA/dominance + ownership/async invariants
  const_fold.rs const_prop.rs branch.rs copy_prop.rs dce.rs
  drop.rs rc.rs move_prop.rs
  escape.rs sroa.rs copy_elision.rs
  bounds.rs collections.rs
  async_opt.rs closure.rs
```

Pipeline: `verify → canonicalize → const_prop/fold → branch → copy_prop → dce →
drop/rc → escape/sroa → bounds → async/closure → verify`. `-O0` runs no passes.
Each pass is deterministic, independently tested, and preserves ownership, Drop,
and async suspension semantics.

## 5. Optimization UX

Implemented in M2.4.7:

- `vut build --release` is kept and maps to `-O2`.
- `OptimizationLevel` (`O0..O3`) lives on `CompilerConfig`; Debug defaults to
  `O0`, Release to `O2`.
- `vut build -O0|-O1|-O2|-O3` / `--opt-level` is accepted; an explicit `-O`
  overrides the build-mode default.
- `-O0` runs no MIR passes and is the correctness/reference path for differential
  testing.

Deferred: a dedicated size mode (`-Os`/`-Oz`) mapping to Cranelift
`speed_and_size`.

## 6. Sub-phases

### M2.4.1 — Benchmark & measurement infrastructure

- Goal: reproducible corpus + regression harness; no optimizer behavior change.
- Problem: only process-level `prime_count`; no corpus/percentiles/compile timing.
- Files: `benchmarks/` (Rust runner + corpus), `benchmarks/baseline.json`.
- Tasks: corpus (numeric, function, collections, memory/RC, optional/result,
  closure, concurrency/channel, FFI); runner compiles each `.vut` with
  `--release`, warmup + N iterations, reports median/p95/variance, verifies
  correctness, writes JSON history; compile-time breakdown harness; regression
  diff. Keep the existing PowerShell benchmarks until the new runner fully
  replaces them, then remove.
- Tests: runner self-test; per-benchmark correctness assertions.
- Benchmarks: the corpus.
- Risks: noise/startup domination → inner-loop amortization + medians.
- DoD: one command runs the corpus locally; baseline JSON committed; correctness
  verified.
- Depends: none.

### M2.4.2 — MIR verifier + canonicalization + safe basics

- Goal: pass framework + verifier + provably-safe passes; fix §0 first.
- Problem: monolithic pass; no verifier; DCE can drop Spawn/AwaitFuture.
- Files: `crates/vut-mir/src/optimize/*`, `vut-mir/src/lib.rs`.
- Tasks: pipeline driver + report; verifier (def-use, dominance, move-once,
  drop-of-moved, retain/release balance, await/child invariants, frame slots);
  **complete the side-effect model so Spawn/AwaitFuture/StartFuture/CallIndirect/
  InterfaceCall/OptionalFromValue are never removed**; constant propagation;
  branch folding + unreachable-block removal; copy propagation; fixed-point DCE;
  run verifier before/after passes.
- Tests: unit per pass; verifier positive/negative; all e2e in release; a
  regression test proving discarded `vut(...)` still spawns.
- Benchmarks: `prime_count` must not regress.
- Risks: ownership/async breakage → verifier + differential gate.
- DoD: pipeline + verifier green; release differential clean; no semantic change.
- Depends: M2.4.1.

### M2.4.3 — Ownership / Drop / RC optimization

- Goal: cut Drop/Retain/Release/MakeUnique traffic without changing destruction.
- Problem: no elimination of redundant RC/drop; copy where move is legal.
- Files: `optimize/drop.rs`, `optimize/rc.rs`, `optimize/move_prop.rs`.
- Tasks: drop-flag/conditional-drop elimination; retain→release pairing removal
  when refcount is provably stable; MakeUnique elision when uniquely owned; move
  propagation; never remove observable drops or cross-thread-visible RC ops.
- Tests: `ownership_e2e`, `value_semantics_e2e`, leak-count tests, differential.
- Benchmarks: `str`/`bytes`/`list[T]`/`channel[T]` managed ops.
- Risks: double-free/UAF/leak → verifier + ASan/LSan.
- DoD: measured RC reduction; ownership/leak suites green.
- Depends: M2.4.2.

### M2.4.4 — Escape analysis + allocation/copy elision

- Goal: stack/register non-escaping values; construct-in-place; avoid copies.
- Problem: `escape.rs` is a stub; aggregates copied; no SROA.
- Files: `vut-memory/src/escape.rs` + `optimize/escape.rs`, `optimize/sroa.rs`,
  `optimize/copy_elision.rs`.
- Tasks: escape classification (Local/Returned/Stored/closure/Vutcon/channel/FFI);
  SROA for non-escaping `data`; return/argument/constructor copy elision;
  optional/result payload elision.
- Tests: data-model e2e, differential, sanitizers.
- Risks: lifetime errors → act only on proven non-escape.
- DoD: escape-driven wins with evidence; no semantic change.
- Depends: M2.4.3.

### M2.4.5 — Bounds-check elimination + collection/loop optimization

- Goal: remove provably-safe checks; reduce collection temporaries; preallocate.
- Problem: every `at`/index is a checked runtime call.
- Files: `optimize/bounds.rs`, `optimize/collections.rs`; loop metadata if needed.
- Tasks: induction/range analysis for canonical loops; constant index; length
  hoisting; inline `at` fast path when proven; `reserve` from known length;
  hoist loop-invariant data pointer; avoid per-iteration MakeUnique.
- Tests: bounds e2e; OOB must still trap; differential.
- Risks: removing a needed check → memory unsafety → proof + verifier only.
- DoD: canonical-pattern elimination; traps preserved; benchmark improvement.
- Depends: M2.4.2.

### M2.4.6 — Closure / async / Vutcon / Channel optimization

- Goal: smaller frames/spills; fast paths; stack env for non-escaping closures.
- Problem: 16B spills, 32B iterator regions, per-spawn alloc; heap env always.
- Files: `future/frame.rs`, `future/mod.rs`, `future/spill.rs`,
  `optimize/async_opt.rs`, `optimize/closure.rs`.
- Tasks: exact-size spills; skip non-crossing spills; state compaction;
  non-escaping closure → stack env; await/spawn/completed fast paths; channel
  payload move; preserve `PollChannelRecv → AwaitChannelRecv → continuation →
  OptionalFromValue`.
- Tests: async/vutcon/channel e2e + leak tests.
- Benchmarks: spawn/await throughput; channel send/recv MPMC.
- Risks: suspension correctness → verifier + e2e.
- DoD: smaller frames/spills; async suites green.
- Depends: M2.4.3, M2.4.4.

### M2.4.7 — Backend / release pipeline

- Goal: exploit Cranelift/toolchain; ship `-O` UX.
- Problem: Debug=none, Release=speed; no target CPU; no inlining strategy.
- Files: `cranelift/compile.rs`, `vut-compiler/src/compiler.rs`,
  `vut-cli/src/cli.rs`, vpm manifest, specs.
- Tasks: `OptimizationLevel`/`BuildProfile` (`-O0..-O3`, `--release≡-O2`, size
  mode); target-CPU/features (host default, explicit override); Cranelift
  settings tuning + debug verifier; function-attribute/inlining strategy; record
  binary size/compile time.
- Tests: config tests; differential across levels.
- Risks: compile-time/binary-size blowup; target-cpu portability.
- DoD: documented levels; `-O0` == today's dev; measurements recorded.
- Depends: M2.4.2.

### M2.4.8 — Differential testing + regression gate

- Goal: correctness gate + regression history.
- Files: `crates/vut-compiler/tests/differential_e2e.rs`, `benchmarks/` history,
  `.github/workflows/ci.yml`.
- Tasks: run every benchmark/e2e at `-O0` and optimized, compare stdout+exit;
  commit baseline; thresholds for runtime/alloc/binary-size/compile-time; wire
  differential correctness into CI; keep the full performance regression suite
  manual/local or nightly (not every CI invocation).
- DoD: differential suite green in CI; regression report; targets met.
- Depends: all prior phases.

## 7. Test Strategy

Per pass: same observable result; Drop/trap/ownership/side-effect/FFI/Optional/
Result/closure/async/Vutcon/channel preserved; run the MIR verifier after each
transformation. Differential `-O0` vs optimized is the strongest gate.

## 8. Benchmark Strategy

Rust-based runner; release build; same hardware/target; warmup; ≥15 iterations;
median + p95 + variance; no I/O in CPU benchmarks; prevent dead-code elimination
of the measured work; verify results; persist JSON baseline/history; measure
runtime, allocation where feasible, compile time, and binary size.

## 9. Correctness Strategy

MIR verifier after each pass; differential `-O0` vs optimized in CI; existing
ownership/leak suites; ASan/LSan (already in CI) extended to optimized E2E.

## 10. Performance Targets

- Semantic parity: differential 100% (hard gate).
- `prime_count`: no regression > 5%.
- Collection/RC/async benchmarks: measurable improvement vs M2.4.1 baseline.
- Release compile time: optimizer passes add < ~25% (measured in M2.4.1).
- Binary size: no growth > 20% without documented justification.

## 11. Definition of Done

Corpus + baseline committed; verifier + pass framework; must-have passes landed
and verified; differential gate green at every level; all existing tests + fmt +
clippy + sanitizers green; compile-time/binary-size within targets; the DCE
blocker (§0) fixed with a regression test; docs updated.

## 12. Deferred Work

- Escape-driven stack allocation and full SROA of non-escaping aggregates.
- Async frame compaction beyond reload dedup (state compaction, per-type spill
  sizes) and non-escaping closures without a heap environment.
- Target-CPU/feature selection and an explicit inlining strategy.
- A dedicated size optimization mode (`-Os`/`-Oz`).
- The Rust benchmark corpus/history and the performance regression gate
  (M2.4.1/M2.4.8 performance half); shared-CI timing is not a hard gate.
- LLVM backend (separate roadmap), custom loop vectorizer/unroller, map/string
  data-structure redesign, JIT/GC, new syntax/borrow checker/scheduler,
  `for value in ch:` iteration (channel follow-up).

## 13. Risks

Optimizer-induced UB (verifier + differential + sanitizers); async/ownership
breakage; compile-time/binary-size blowup; target-CPU portability; measurement
noise; under-tested release path until M2.4.2/M2.4.8.

## 14. Classification

- MUST HAVE: verifier + pass framework; DCE side-effect fix; constant/branch/copy
  propagation; drop/RC/move optimization; bounds-check elimination; differential.
- HIGH VALUE: escape/SROA/copy elision; async/closure frame reduction; target-CPU
  + inlining; channel/collection optimization; compile-time instrumentation.
- CRANELIFT ALREADY HANDLES: scalar folding, register allocation, instruction
  selection, per-function DCE, block layout.
- DEFER: custom vectorizer, map/string redesign, LLVM backend, JIT/GC, new
  syntax/borrow checker/scheduler.

## Verification

Planning task only. No optimization implemented. `cargo fmt --check`,
`cargo clippy --workspace --all-targets`, and `cargo test --workspace` remain
clean; this document adds no code.
