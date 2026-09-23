# M2.6 — Advanced Optimization & Performance Finalization

M2.6 is Vut's final large optimizer/codegen build-out. After M2.6, future
performance work should be **incremental**: target-specific tuning, regression
fixes, or new optimizations on top of a mature architecture — not another
redesign milestone.

The goal is **not** to claim Vut is faster than C/Rust/Zig. The goal is to build
a benchmark/measurement system and real data showing Vut is
**performance-competitive on its target workloads**.

Backend note: the Vut native backend is **Cranelift 0.135.2**, not LLVM. M2.6 is
Cranelift-only. M2.4 already established the optimizer baseline (O0–O3,
bounds-check elimination, async optimization, differential testing), so M2.6
must not blindly re-implement what Vut or Cranelift already does well.

Status: **In Progress.** M2.6.1 (benchmark & measurement infrastructure) and
M2.6.2 (MIR analysis framework + pass manager + verifier hardening) and M2.6.3
(SCCP + MIR CSE / value propagation), M2.6.4 (DCE/ADCE + dead store +
store-to-load forwarding), and M2.6.5–M2.6.10 (loop LICM + bounds II, escape
analysis, SROA, RC II, string folding, call graph + expression inlining) are
complete; **M2.6 is complete** (M2.6.1–M2.6.18).

### M2.6.11–M2.6.18 — completed

- **M2.6.11** `analyze/devirt.rs`: devirtualization analysis (known-receiver and
  single-implementor interface calls). Rewriting `InterfaceCall` needs an
  interface-data MIR operation; deferred.
- **M2.6.12** `CallGraph::reachable_from`: whole-program reachability analysis.
  Dead-function removal needs the entry symbol at `optimize` time; deferred.
- **M2.6.13** `optimize/closure.rs`: devirtualizes `CallIndirect` to a direct
  `Call` when the callee is a known non-capturing function value.
- **M2.6.14** `optimize/async_opt.rs`: reload dedup (existing), overwritten
  frame-state removal, and dead scalar-spill removal. Frame compaction deferred.
- **M2.6.15** target CPU/features: `Target { triple, cpu, features }`,
  `CompilerConfig.target_cpu/target_features`, CLI `--target-cpu`
  (`native`/`x86-64-v2/v3/v4`) and `--target-feature ±feat`, host feature
  detection, and `VUT_CL_VERIFIER` to toggle the Cranelift verifier. Portable
  baseline remains the default; native is opt-in and independent of `-O`.
- **M2.6.16** SIMD enablement via M2.6.15 + feasibility report
  (`specs/compiler/vectorization-feasibility.md`); no auto-vectorizer.
- **M2.6.17** PGO feasibility + design report
  (`specs/compiler/pgo-feasibility.md`); no PGO implementation.
- **M2.6.18** certification gate: `.github/workflows/perf.yml` (nightly/on-demand
  corpus run) and `benchmarks/CERTIFICATION.md` with the win/tie/loss table and
  compiler/runtime-level gap explanations.

### M2.6.5–M2.6.10 — completed

- **M2.6.5** `optimize/loops.rs`: loop-invariant code motion for pure,
  non-trapping scalar ops (`Binary` except div/rem, `Unary`, `*Len`) hoisted to
  the preheader. Bounds II: reversed comparison facts (`len > counter`) and a
  variadic loop-in-bounds fact. Induction-variable/strength-reduction and general
  loop canonicalization are deferred.
- **M2.6.6** `analyze/escape.rs`: `EscapeSummary` (call/closure/spawn/store/
  interface/return escape) with transitive propagation. Delivered as the shared
  escape analysis; deep consumption (stack promotion) is deferred.
- **M2.6.7** `optimize/sroa.rs`: scalar-replaces a non-escaping `Construct`
  whose only uses are `Field` reads (copyable, non-managed aggregates). Stack
  promotion of `Allocate` results is deferred (needs a place abstraction).
- **M2.6.8** `optimize/rc.rs`: retain/release cancellation now crosses
  unconditional `Jump` edges (unique-path chains); joins reset the pending set.
- **M2.6.9** `optimize/strings.rs`: folds `ConstString + ConstString` at compile
  time. Runtime string-builder/ABI work is deferred.
- **M2.6.10** `analyze/callgraph.rs` (direct edges, address-taken, indirect
  presence) + `optimize/inline.rs`: inlines single-block, pure, scalar-only
  functions at direct call sites. Cost-model/aggressive inlining and WPO remain
  M2.6.11/M2.6.12.
- Baseline comparison after these phases: deterministic structural reductions
  (`inst-1` on map/prime_count/vutcon) and no semantic change; the small
  micro-corpus does not heavily exercise these passes (corpus expansion is part
  of M2.6.18).

### M2.6.4 — completed

- `copy_prop.rs` now tracks `Store` into copyable locals, giving store-to-load
  forwarding (`Store L,v; Copy w,L` ⇒ `w` aliases `v`).
- New `optimize/memory.rs`: intra-block dead-store elimination for copyable,
  non-managed scalar locals (never removes an owned/managed store).
- `OptimizationReport::dead_stores_removed` added.
- Baseline comparison after M2.6.3+M2.6.4: structural MIR counts reduced
  (`inst-1` on map/prime_count/vutcon) with no semantic change; wall-clock
  deltas on the small corpus remain within noise.

### M2.6.3 — completed

- `optimize/sccp.rs` replaces `const_prop.rs`: constant folding, value-preserving
  algebraic identities (`numeric::simplify`: `x+0`, `x*1`, `x*0`, `x/1`,
  boolean `and`/`or`, integer self-comparisons), constant propagation through
  copyable scalar locals, and constant-branch folding. Integer/boolean only —
  float identities are not value-preserving.
- `optimize/cse.rs`: intra-block CSE for the pure `*Len` builtins (opaque to
  Cranelift), cleared by any observable instruction.
- `OptimizationReport::cse_eliminated` added.

### M2.6.2 — completed

- New `crates/vut-mir/src/analyze/`: `Cfg`, `DominatorTree`, `DefUse`,
  `Liveness`, `LoopInfo` (reusable, deterministic). `bounds.rs` now uses the
  shared CFG/dominator analyses instead of private copies.
- New `optimize/manager.rs`: phase-based `PassManager` with a bounded fixpoint;
  each phase verifies its output and reverts to its input on failure (recorded
  as `OptimizationReport::reverted_phases`).
- Verifier hardening: block/local ranges, operand definitions, **dominance** of
  every operand, and an ownership-state **move-once / use-after-move /
  drop-of-moved** dataflow (conservative at joins).
- `OptionalFromValue` documented as pure (no allocation/side effect); the older
  roadmap note claiming otherwise was inaccurate.

### M2.6.1 — completed

- New `crates/vut-bench` (bin + lib): corpus discovery, compile/run at O0–O3,
  wall time (trimmed median, robust to interference), compile time, binary size,
  deterministic MIR structure metrics, JSON baseline + `compare`, and a
  best-effort C/Rust/Zig/Go competitor harness.
- `benchmarks/corpus/` (15 workloads) with verified `expected.txt`; committed
  `benchmarks/baseline.json` (schema 2) and `benchmarks/BASELINE.md`.
- Structure metrics are collected from MIR **after** `vut_mir::optimize` at the
  requested level, so they gate optimizer regressions (e.g. `list` bounds 1→0).
- Baseline findings recorded for later phases: shallow optimizer reach,
  `tight_loop`/`recursion` gaps vs Rust, `prime_count` parity.

---

## 0. Locked decisions

1. **PGO (M2.6.17)** requires a **feasibility report + architecture/design**.
   Minimal PGO is implemented only if the spike shows clean integration, clear
   benefit, and no scope blow-up. Full PGO is **not** a hard DoD.
2. **SIMD (M2.6.16)** is **enablement + generated-code verification +
   vectorization feasibility report**. No auto-vectorizer in M2.6.
3. **Map optimization is ABI-stable first.** Do not redesign the typed-map
   representation unless M2.6.1 benchmarks prove the current representation is a
   major bottleneck and ABI-stable optimizations are insufficient. If that
   happens, **stop and report before changing the ABI.**
4. **Benchmark hard comparison set is C + Rust.** Zig + Go are best-effort when
   the toolchain/environment is available; missing Zig/Go must never fail M2.6.
5. **Wall-clock regression runs nightly/on-demand on a dedicated runner**, never
   on noisy shared runners to fail PRs. Deterministic structural metrics
   (allocation count, RC operations, remaining bounds checks, frame size, MIR
   instruction metrics) may be used as PR regression gates where appropriate.
6. `crates/vut-bench` is approved as a separate crate.

### Mandatory corrections

- **Optimization level and CPU target are independent dimensions.** O0/O1/O2/O3
  all default to the **portable CPU baseline**. `--release`/O2 and O3 do **not**
  implicitly enable native CPU features. Native optimization happens only when
  the user explicitly passes `--target-cpu native` (or a named target preset).
  A release binary must be portable within the published target baseline by
  default.
- **Verification policy is not tied to optimization level.** In development,
  tests, and CI, **both the MIR verifier and the Cranelift verifier are enabled
  for O0/O1/O2/O3**. The production compiler, once M2.6 is stable, may disable
  the Cranelift verifier to reduce compile time and must expose an option/env to
  re-enable it. The Cranelift verifier must never be implicitly toggled by `-O`.
- **RC/ownership verification must not use a naive "retains == releases" rule.**
  The verifier/dataflow must model ownership **states and transfer semantics**:
  `owned`, `borrowed`, `moved`, `transferred`, `escaped`, `released`, and must
  treat CFG joins, calls, returns, containers, closures, Vutcon/async, and FFI
  boundaries conservatively.

### Certification requirement

The final M2.6 certification report must explain every large performance gap or
regression **at the compiler/runtime level** — for example allocation, RC/COW,
bounds checks, indirect calls, runtime calls, hashing, async frames, or codegen —
and not merely present win/loss numbers.

---

## 1. Current state (audited)

- Pipeline: `source → lexer → parser/AST → resolver → type checker → HIR → MIR
  (ownership/drop + monomorphization) → async/Vutcon lowering → MIR optimization
  → Cranelift → object → linker → executable`.
- Optimizer: 14 passes, sequential, **verify-after-only**, whole-batch revert
  (`crates/vut-mir/src/optimize/mod.rs:99-139`). O1 ≡ O2; O3 adds one cheap round.
- No reusable analyses: `successors` duplicated in 5 files; private dominators
  only in `bounds.rs:245-282`; no CFG/liveness/loop/def-use types.
- `optimize/escape.rs` is built + exported but **called by nothing**;
  `sroa.rs` only removes unused `Allocate`; `rc.rs`/`drop.rs`/`move_prop.rs` are
  intra-block; `async_opt.rs` only deduplicates frame reloads.
- Verifier checks only block-id range, operands "defined somewhere", local index
  range (`optimize/verify.rs`).
- Codegen: only `is_pic=true` + `opt_level=speed` in optimizing builds
  (`cranelift/compile.rs:39-55`); ISA built via `isa::lookup(triple).finish(flags)`
  with **no CPU features** → baseline SSE2. Cranelift **0.135.2**.
- Cranelift already does (do not duplicate): egraph constant folding/GVN/CSE,
  algebraic simplification, per-function DCE, block layout, redundant-load
  elimination via alias analysis (`enable_alias_analysis` default true).
  Cranelift has **no inliner, no auto-vectorizer, no LICM, no interprocedural
  pass, no profile input**.
- Runtime: `str` double refcount; `map[K,V]` = `HashMap<Vec<u8>,Vec<u8>>` +
  SipHash + two allocs/entry; string concat ≈ 3 allocs; async frames raw blocks
  with 8-byte spills; per-spawn frame+task+handle+registry allocations.
- Benchmarks: `prime_count.vut/.rs`, `compare.ps1`, `incremental.ps1`, and three
  print-style crate benches. No corpus, no JSON, no CI perf tracking.

---

## 2. Final roadmap

| Phase | Name | Depends |
|---|---|---|
| M2.6.1 | Benchmark & measurement infrastructure | — |
| M2.6.2 | MIR analysis framework + pass manager + verifier hardening | 1 |
| M2.6.3 | SCCP + MIR CSE / value propagation | 2 |
| M2.6.4 | DCE/ADCE + dead store + store-to-load forwarding | 3 |
| M2.6.5 | Loop canonicalization + LICM + induction/strength reduction + bounds II | 2, 4 |
| M2.6.6 | Escape analysis | 2 |
| M2.6.7 | SROA + stack allocation | 6 |
| M2.6.8 | Ownership / RC / COW optimization II | 7 |
| M2.6.9 | Allocation & buffer optimization | 8 |
| M2.6.10 | Call graph + advanced inlining | 2, 5 |
| M2.6.11 | Devirtualization + interface specialization | 10 |
| M2.6.12 | Whole-program optimization | 11 |
| M2.6.13 | Closure optimization | 7, 10 |
| M2.6.14 | Vutcon / async optimization II | 7, 8 |
| M2.6.15 | Target CPU / feature + codegen settings | 2 |
| M2.6.16 | SIMD enablement + vectorization feasibility | 15 |
| M2.6.17 | PGO feasibility + minimal PGO | 10, 15 |
| M2.6.18 | Performance certification gate | all |

Baseline adjustments vs the original 20-phase list: merged Stack+SROA (4+5),
merged SCCP+GVN/CSE (8+9), merged bounds II into loops (10), re-scoped SIMD
(17) and PGO (19) to feasibility. Full rationale is in the plan discussion.

### Dependency graph

```text
1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14 → 15 → 16 → 17 → 18
```

M2.6.15 has no hard dependency on 3–14 and may be pulled forward after M2.6.2.

---

## 3. MIR analysis architecture

```text
crates/vut-mir/src/analyze/
  mod.rs            # AnalysisManager, epoch-based invalidation
  cfg.rs            # successors/predecessors, RPO, reachability
  dominators.rs     # DominatorTree, idom, dominance frontier
  loops.rs          # natural loops, headers, latches, induction candidates
  def_use.rs        # value -> def; value -> uses; operand rewriting
  liveness.rs       # per-block live-in/out for values and frame slots
  effects.rs        # generalized effect queries
  alias.rs          # conservative local/aggregate/pointer alias classes
  constness.rs      # SCCP lattice over SSA values + locals
  callgraph.rs      # M2.6.10: callee sets, address-taken/indirect sets
  escape.rs         # M2.6.6: interprocedural escape classification
```

- Analyses are memoized per `(function, epoch)`; a pass that mutates MIR bumps
  the epoch. `vut-memory/src/escape.rs` is reconciled into one canonical
  implementation.
- Deterministic iteration only (`Vec`/`BTreeMap`); no `HashMap` iteration in
  pass logic.

## 4. Pass manager + fixpoint

```text
crates/vut-mir/src/optimize/{mod.rs, manager.rs, pass.rs}
```

- `Pass::run -> Changed`; `Phase` groups passes and marks cheap phases.
- Bounded fixpoint: cheap phases iterate to a cap (2 at O2, 3 at O3); expensive
  phases (inlining, WPO, loops) run once in fixed order.
- Per-function work budget guards compile time.
- Verify before/after each phase; revert **only** the failing phase.
- Determinism: identical input ⇒ byte-identical MIR (hash test).

## 5. O0–O3 pass matrix

O0 runs no MIR passes and is the reference path. O1 = cheap passes. O2 = full
MIR passes (release default). O3 = O2 + aggressive interprocedural work.

| Pass / setting | O0 | O1 | O2 | O3 |
|---|---|---|---|---|
| const/copy prop, branch fold | – | ✓ | ✓ | ✓ |
| DCE/ADCE | – | ✓ | ✓ | ✓ |
| SCCP + MIR CSE (builtins) | – | basic | ✓ | ✓ |
| dead store / store-to-load | – | – | ✓ | ✓ |
| drop/rc/move | – | intra-block | full | full |
| bounds elimination | – | canonical | general | general |
| loop canonicalize / LICM / IV / strength-reduce | – | – | ✓ | ✓ |
| escape / SROA / stack promotion | – | – | ✓ | ✓ |
| allocation fusion / buffer opt | – | – | ✓ | ✓ |
| inlining | – | – | cost-based | aggressive |
| devirtualization | – | – | basic | full |
| WPO reachability + known-callee | – | – | ✓ | ✓ |
| closure / vutcon II | – | – | ✓ | ✓ |
| MIR verifier | ✓ | ✓ | ✓ | ✓ |
| Cranelift verifier | ✓ | ✓ | ✓ | ✓ |
| Cranelift `opt_level` | none | none | speed | speed |
| CPU target | portable | portable | portable | portable |
| native CPU (opt-in only) | `--target-cpu native` | same | same | same |

Verification is independent of the optimization level (see §0).

## 6. Sub-phases

### M2.6.1 — Benchmark & measurement infrastructure

- Goal: reproducible corpus + history + competitor comparison; no optimizer
  behavior change.
- Files: new `crates/vut-bench/` (bin + lib), `benchmarks/corpus/**`,
  `benchmarks/baseline.json`.
- Architecture: `model` (serde data), `corpus` (discovery), `runner`
  (compile + run at O0–O3), `structure` (deterministic MIR metrics), `metrics`
  (timing/RSS), `report` (JSON + compare), `main` (CLI).
- Metrics: wall time (median/p95), estimated CPU time, peak RSS, startup
  latency, compile time, binary size, exit code, stdout correctness, and MIR
  structural metrics (functions, instructions, allocations, Retain/Release/
  MakeUnique/Drop, runtime calls, checked vs unchecked accesses, interface/
  indirect/direct calls, frame count/bytes).
- Correctness: each workload stores `expected.txt`; a mismatch fails the run.
- Competitor methodology: C `-O2/-O3` and Rust `release` are the hard set; Zig/
  Go best-effort. Same algorithm/input/IO. `black_box`/volatile sink.
- Tests: runner self-test; per-workload correctness assertion; JSON round-trip;
  compare logic.
- Benchmarks: the corpus itself.
- DoD: one command runs the corpus locally; `baseline.json` committed; every
  workload verified; `compare` works; structural metrics recorded.
- Depends: none.

### M2.6.2 — MIR analysis framework, pass manager, verifier hardening

- Goal: reusable analyses + bounded fixpoint driver + a verifier that models
  ownership states/transfers; fix `escape.rs` dead code and the
  `OptionalFromValue` effect mismatch.
- Files: `analyze/*` (new), `optimize/manager.rs`, `optimize/pass.rs`,
  `optimize/verify.rs`, `optimize/mod.rs`, `optimize/effects.rs`.
- Verifier: dominance, move-once, drop-of-moved, Retain/Release ownership-state
  balance (not naive counting), `Allocate` pairing, await/child/frame-slot
  invariants; conservative at CFG joins, calls, returns, containers, closures,
  Vutcon/async, and FFI.
- DoD: analyses tested; verifier stronger; per-phase revert; O0–O3 differential
  green; determinism test; MIR + Cranelift verifiers on at all levels.
- Depends: 1.

### M2.6.3 — SCCP + MIR CSE / value propagation

- Goal: sparse conditional constant propagation over SSA values and locals; CSE
  for opaque Vut builtin calls Cranelift cannot CSE.
- Files: `analyze/constness.rs`, `optimize/sccp.rs`, `optimize/cse.rs`,
  `optimize/numeric.rs`.
- DoD: cross-block constant folding; redundant builtin removal; traps preserved.
- Depends: 2.

### M2.6.4 — DCE/ADCE + dead store + store-to-load forwarding

- Goal: remove dead stores; forward just-stored loads; ADCE via liveness.
- Files: `optimize/dce.rs`, new `optimize/memory.rs`.
- DoD: measured dead-store reduction; no semantic change; FFI/frame stores kept.
- Depends: 3.

### M2.6.5 — Loop canonicalization, LICM, induction/strength reduction, bounds II

- Goal: general bounds-check elimination; loop-invariant hoisting; induction and
  strength simplification.
- Files: `analyze/loops.rs`, `optimize/loops.rs`, rewrite `optimize/bounds.rs`.
- DoD: proof-only unchecked rewrites; every non-provable access still traps;
  measured loop improvement.
- Depends: 2, 4.

### M2.6.6 — Escape analysis

- Goal: classify Local/Returned/Stored/PassedToUnknown/Captured/Sent/FFI; make
  escape facts consumed.
- Files: `analyze/escape.rs`, `optimize/escape.rs`, reconcile
  `vut-memory/src/escape.rs`.
- DoD: consumed by SROA/stack/RC; negative tests for escaping values.
- Depends: 2.

### M2.6.7 — SROA + stack allocation

- Goal: scalar-replace non-escaping aggregates; stack-promote non-escaping
  allocations; construct in place.
- Files: `optimize/sroa.rs`, new `optimize/stack.rs`, `analyze/alias.rs`.
- DoD: measured heap-allocation reduction; sanitizers green; no lifetime errors.
- Depends: 6.

### M2.6.8 — Ownership / RC / COW optimization II

- Goal: interprocedural retain/release cancellation, release sinking, uniqueness
  propagation, COW `MakeUnique` elision.
- Files: `optimize/rc.rs`, `optimize/drop.rs`, `optimize/move_prop.rs`, new
  `optimize/uniqueness.rs`.
- DoD: measured RC/COW reduction; leaks zero; no cross-thread regression.
- Depends: 7.

### M2.6.9 — Allocation & buffer optimization

- Goal: cut allocations in string build, map lookup, list growth, temporaries.
- Files: `optimize/alloc.rs` (new), runtime `abi/string.rs`, `string.rs`,
  `abi/map.rs`, `abi/list.rs`.
- Policy: ABI-stable first (see §0.3). Map representation redesign only if
  benchmarks prove it is the dominant bottleneck and ABI-stable work is
  insufficient — and only after reporting to the user.
- DoD: alloc counts measurably reduced; string/map/list benchmarks improve; ABI
  unchanged or bumped with an explicit report.
- Depends: 8.

### M2.6.10 — Call graph + advanced inlining

- Goal: cost-model-based interprocedural inlining (Cranelift has none).
- Files: `analyze/callgraph.rs`, `optimize/inline.rs`, `optimize/cost.rs`.
- DoD: capped cost-based inlining; measurable wins; compile-time budget met.
- Depends: 2, 5.

### M2.6.11 — Devirtualization + interface specialization

- Goal: direct calls for statically-known vtables; single-implementor
  specialization.
- Files: `optimize/devirt.rs`, `analyze/callgraph.rs`.
- DoD: proof-only devirtualization; behavior unchanged.
- Depends: 10.

### M2.6.12 — Whole-program optimization

- Goal: reachability-based dead function elimination; whole-program
  constant/known-callee propagation.
- Files: `optimize/wpo.rs`, `analyze/callgraph.rs`.
- DoD: unreachable functions removed safely; FFI/exports/vtables preserved.
- Depends: 11.

### M2.6.13 — Closure optimization

- Goal: no heap environment for non-escaping closures; cheaper closure calls.
- Files: `optimize/closure.rs`, `cranelift/instruction.rs`, runtime closure path
  if needed.
- DoD: non-escaping closures allocate nothing; escaping semantics unchanged.
- Depends: 7, 10.

### M2.6.14 — Vutcon / async optimization II

- Goal: smaller frames, fewer allocations, fast paths.
- Files: `optimize/async_opt.rs`, `lowering/future/{frame,spill,machine}.rs`,
  runtime frame/task/async-handle/vutcon.
- DoD: smaller frames/spills, fewer allocs measured; async suites green.
- Depends: 7, 8.

### M2.6.15 — Target CPU / feature + codegen settings

- Goal: allow native CPU optimization **explicitly**, without changing the
  portable default.
- Files: `cranelift/compile.rs`, `vut-codegen/src/cranelift.rs`,
  `CompilerConfig`, `vut-cli/src/cli.rs`, vpm `project.rs`, specs.
- Policy: independent of optimization level; default portable; `--target-cpu
  native|<preset>` and `--target-feature ±feat` opt in; Cranelift verifier
  toggle-able for production only; optional `-Os` size mode.
- DoD: portable default verified; native opt-in works; documented; differential
  green.
- Depends: 2.

### M2.6.16 — SIMD enablement + vectorization feasibility

- Goal: use SIMD where Cranelift already lowers it; produce a vectorization
  feasibility report.
- Files: `optimize/simd.rs` (safe patterns only), `specs/compiler/optimization.md`.
- DoD: SIMD used for bulk/`memcpy`/`popcnt`-style patterns; written go/no-go
  report. **No auto-vectorizer.**
- Depends: 15.

### M2.6.17 — PGO feasibility + minimal PGO

- Goal: feasibility report + architecture/design for PGO under Cranelift.
- Finding: Cranelift exposes no profile primitives; PGO would be Vut-owned end
  to end (instrumentation, profile format, consumer, and a way to act on it).
- DoD: written feasibility report + design; implement minimal inlining-weight
  PGO only if clean and clearly beneficial. Full PGO not in DoD.
- Depends: 10, 15.

### M2.6.18 — Performance certification gate

- Goal: certify the architecture is mature and future work is incremental.
- Files: `crates/vut-bench/`, `.github/workflows/perf.yml`,
  `benchmarks/baseline.json`, specs.
- DoD: see §9.

---

## 7. Benchmark architecture

- `crates/vut-bench` CLI: `run`, `compare`, `list`.
- Corpus under `benchmarks/corpus/<name>/` with `main.vut`, `expected.txt`, and
  optional `stdin.txt`.
- Metrics per (workload, level): wall (median/p95), estimated CPU, peak RSS,
  startup latency, compile time, binary size, exit code, stdout match, and MIR
  structural metrics.
- History: timestamped JSON runs plus committed `benchmarks/baseline.json`.
- Comparison: win/tie/loss and percentage deltas per workload; the certification
  report must include losses with compiler/runtime reasons.

## 8. Regression CI design

- PRs (3 OSes): correctness only — differential O0–O3, verifier/property tests,
  leaks/sanitizers. **Deterministic structural metrics** (allocation count, RC
  ops, checked bounds, frame size, MIR instruction counts) may gate PRs because
  they are noise-free.
- Nightly / on-demand (dedicated runner): wall-clock corpus run vs baseline with
  relative thresholds; artifacts + history; never fail PRs on shared runners.
- Compile-time budget and binary-size budgets recorded and guarded.

## 9. Certification criteria (M2.6 DoD)

1. Windows/Linux/macOS CI green.
2. O0/O1/O2/O3 output-equivalent on the full corpus + e2e.
3. Verifier strengthened; per-phase fail-safe revert proven.
4. Memory safety/leak tests green on optimized builds.
5. Full corpus + committed historical baseline + regression thresholds.
6. Published win/tie/loss table vs C/Rust (Zig/Go best-effort) with
   compiler/runtime explanations for every large gap.
7. Portable CPU default verified; native opt-in documented.
8. No known major optimizer architectural blocker remains.
9. Escape/SROA/inlining/devirtualization/async-II landed with measurements.
10. SIMD and PGO feasibility reports delivered with go/no-go conclusions.

## 10. Risks

Miscompilation (verifier + differential + sanitizers + per-phase revert);
analysis invalidation bugs (epoch model + tests); compile-time explosion
(fixpoint/work budgets); target-feature portability (portable default); runtime
ABI churn (prefer ABI-stable, report before bumping); benchmark noise
(medians/p95, dedicated runner); PGO/vectorization infeasibility (spike first).

## 11. Non-goals (Cranelift already handles)

Scalar folding/GVN/CSE, per-function DCE, block layout, alias-based redundant
loads, register allocation, instruction selection, legalization, trap lowering,
bulk-memory selection, loop unrolling/scheduling.

## Verification

Per-phase: `cargo fmt --check`, `cargo clippy --workspace --all-targets`,
`cargo test --workspace --all-targets` must be clean before the next phase.
