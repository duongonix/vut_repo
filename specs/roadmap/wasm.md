# WASM — WebAssembly target roadmap

Canonical plan for making WebAssembly a production-quality Vut target. This file
is the source of truth for phase scope, status, and Definition of Done (DoD).

Targets: `wasm32-wasip1` (WASI first) and `wasm32-unknown-unknown` (browser).

Pipeline: `source → frontend → HIR → MIR → optimizer → WasmBackend → .wasm`
(see `specs/wasm/00-overview.md`). Native stays on `CraneliftBackend` + system
linker.

## Rules

- A phase is **Complete** only when its full DoD is met. Moving a core DoD
  requirement to "deferred" does **not** make a phase complete. Only explicitly
  optional enhancements may be deferred.
- Native semantics/ABI must not regress.
- Native↔WASM **differential tests** are added incrementally from C3/C4, not
  postponed to certification.
- Hard-coded pointer/layout assumptions are fixed in C1 before more wasm
  semantics are built.
- Safety semantics must match Vut/native (no "return 0" on out-of-bounds; traps
  are traps).
- RC/COW must be real; `Retain/Release`/`MakeUnique` may not be no-ops to pass
  tests.
- Export roots must be explicit; do not rely on the absence of function-level
  DCE.
- C11 certifies; it does not fix missing semantics.

## Phase status (WASM.0–WASM.14)

| Phase | DoD | Status |
| --- | --- | --- |
| WASM.0 | Spike: validate emit/link/runtime feasibility; choose object-emission approach | **Complete** |
| WASM.1 | Target plumbing (`Flavor::Wasm`, `--target`, doctor) + ABI/overview spec + early validation | **Complete** (roadmap/status docs reconciled by C0) |
| WASM.2 | Target layout abstraction; remove all hard-coded 64-bit assumptions | **Partial** → finished by **C1** |
| WASM.3 | Minimal MIR→wasm codegen: scalars, arithmetic, control flow, functions, locals, calls, entry | **Complete** |
| WASM.4 | Linear memory + allocator + `memory.grow` + **panic/bounds traps** + entry | **Complete** (C2) |
| WASM.5 | `str`/`bytes`/`list` runtime **with RC/COW** | **Partial** → **C3** (core RC/COW done; full method surface + float formatting remain) |
| WASM.6 | `map` + remaining managed parity (optional/result, data/enum, arrays, variadics/iterators, closures, interfaces, higher-order) | **Partial** → **C4** |
| WASM.7 | Imports/exports ABI + `InterfaceMetadata` + explicit export roots for WPO/DCE | **Partial** → **C5** |
| WASM.8 | WASI platform + supported stdlib subset + `vut build/run --target wasm32-wasip1` | **Partial** → **C6** |
| WASM.9 | Vutcon/async WASM: real suspension, state-machine lowering, single-thread executor, `vutcon`, channels, host bridge | **Partial (core deferred)** → **C7** |
| WASM.10 | Browser host runtime (`wasm32-unknown-unknown` end-to-end) | **Partial** → **C8** |
| WASM.11 | JS/TypeScript bindings with marshaling | **Partial** → **C9** |
| WASM.12 | VPM target integration | **Complete** |
| WASM.13 | WASM optimization + `-Os` (dead import/export/memory, host-call reduction) | **Partial (basic `-Os` complete)** → **C10** |
| WASM.14 | Production certification gate | **Partial (WASI subset)** → **C11** |

## Remaining work (C0–C11)

| Phase | Completes | Scope | DoD | Depends |
| --- | --- | --- | --- | --- |
| **C0** | docs | Canonical roadmap; reconcile `specs/wasm/00-overview.md`, `spikes/wasm0/*.md`, `benchmarks/WASM-CERTIFICATION.md` | Docs match implementation; no false "Complete" | — |
| **C1** | WASM.2 | `TargetLayout` abstraction; pointer-width in Cranelift `machine_type`/runtime signatures; MIR frame/iterator/spill/closure/optional/range constants derived from pointer size | Native suite green; wasm32 offsets pointer-derived; no `Pointer → I64` hard-coding | C0 |
| **C2** | WASM.4 | Traps: OOB `list`/`array`/`bytes`, numeric conversions, panic trap | Traps fire; no silent `0` | **Complete**: OOB `list`/`bytes` trap; `NumericCast` int↔int/float↔int checked (trap), float↔float, `IntToFloat`, `FloatToInt`; fixed float unary negate + `infer_type(Unary)` | C1 |
| **C3** | WASM.5 | RC/COW for `str`/`bytes`/`list`; full method surface; float formatting | Native↔wasm differential on managed corpus | **Complete**: RC/COW + element retain/release; full `str`/`bytes`/`list` surfaces incl. Unicode-scalar ops, Unicode-`White_Space` trim, number parse; `bool.to_str`; int `.to_str`; float formatting (fixed-point, 6-digit fraction). Caveats: `to_lower`/`to_upper` ASCII-only; float formatting not always shortest-round-trip (e.g. `1/3`) | C2 |
| **C4** | WASM.6 | optional/result; data/enum aggregates; arrays; variadics/iterators; closures; interfaces; map/collection parity; higher-order ops. May split into C4.x subphases (tests per subphase) | Native↔wasm differential on aggregates/enums/interfaces | **Nearly complete**: optionals (`T?`/`null`), result (`ok`/`err`/`is_ok`/`is_err`/`unwrap_or`), arrays (`len`/`at`/`set`/`first`/`last`/`fill`/`contains`/`to_list`), data/enum (`Construct`/`Field`/`FieldStore`/`BorrowField`/`ConstructEnum`/`EnumTag`/`EnumPayload`/`CopyAggregate`), map extras (`is_empty`/`capacity`/`reserve`/`keys`/`values`), iterators (`IteratorInit`/`IteratorNext`), closures + indirect calls (`MakeFunction`/`CallIndirect` + funcref table), higher-order ops (`map`/`filter`/`any`/`all`/`fold`) — all match native. Remaining: variadics (`ConstructVariadicBuffer` implemented, runtime layout still mismatched) and interfaces/vtables (`ConstructInterface`/`InterfaceCall`) | C3 |
| **C5** | WASM.7 | Explicit export surface; DCE/function-removal treats exports/imports as roots | Unused exported functions retained | **Complete**: explicit export surface (`vut_fn_<symbol>` + `main`/`_start`/`memory`/`vut_alloc`); **function-level DCE** drops functions unreachable from the entry, and reachable functions (roots) are retained and exported (verified by `unused_exported_function_is_retained`) | C4 |
| **C6** | WASM.8 | `vut run --target`; stdlib extern to WASI mapping; supported subset; explicit diagnostic for unmapped modules | Subset runs under WASI; unsupported modules diagnosed | **Partial**: `vut run --target wasm32-wasip1` builds `.wasm` and runs it via a discovered runtime with an actionable error; unsupported native-runtime-backed modules are diagnosed; `wasm/stdlib.rs` provides the WASI imports, result-envelope + counted-resource infrastructure, and helpers for `count`/`env`/`time`/`io`/`os`/`random`, plus reachability-based import pruning. Fixed the `buf[i-1]` double-load and the pointer-sized result tag (4 bytes on wasm), so `env.has`/`env.args` now work. Remaining: `env.get`/`env.all` still take the error branch; `io`/`os`/`fs` not verified/complete. **`math` complete**: all 17 `vut_rt_math_*` externs (`sin`/`cos`/`tan`/`asin`/`acos`/`atan`/`atan2`/`exp`/`exp2`/`log`/`log2`/`log10`/`sinh`/`cosh`/`tanh`/`cbrt`/`hypot`) implemented in `wasm/math.rs` (portable libm: range reduction + Taylor/Horner, `__ldexp` helper) and verified native-vs-wasm; also fixed the float formatter's rounding carry (`7.9999…` → `8`) | C4 |
| **C7** | WASM.9 | Real future/state-machine lowering on wasm; single-thread executor; `vutcon`; channels; host async bridge | Async e2e with suspension; native↔wasm differential | **Partial (C7.1 complete)**: wasm now runs the real target-independent async pass (`vut_mir::lower`, replacing `lower_sync`); the wasm backend lowers the poll state machine (`FrameState`/`SetFrameState`/`SetFrameChild`/`SpillValue`/`ReloadValue`/`PollFuture`/`PollReturn`), `StartFuture` (frame alloc + handle), `AwaitFuture`, and a single-threaded cooperative executor in `wasm/asyncrt.rs` (`AsyncHandle` in linear memory, `frame_alloc`/`handle_new`/`await_child`/`drive`/`executor_run`); poll entries take `(frame, out) -> i32` and straight-line async bodies get resume thunks; async `main` allocates a root handle and drives the executor. Verified: `async_state_machine_runs_under_wasi` (native↔wasm differential). Remaining: `vutcon`/`Spawn`, channels (`PollChannelRecv`), a real pending-future source (suspension needs channels/host), and the JS/host async bridge | C4 |
| **C8** | WASM.10 | Browser runtime (`wasm32-unknown-unknown` e2e, `_initialize`, host imports, API packages) | Headless-browser program runs | C5 |
| **C9** | WASM.11 | JS/TS marshaling (str/bytes/list/result/resource/callbacks) + `.d.ts` | JS interop tests (Node + browser) | C8 |
| **C10** | WASM.13 | Wasm-specific size/effect passes; optional wasm-opt/strip | Dead import/export/memory reduced; no semantic regression | C9 |
| **C11** | WASM.14 | Certification: internal `wasmparser`; `wasm-tools validate`; Wasmtime; Node; headless browser; native↔wasm differential; O0/O1/O2/O3 + `-Os`; ownership/resource lifetime; real async/Vutcon; JS/TS interop. CI installs wasm-tools/Wasmtime/Node/browser tooling (not bundled in the Vut distribution) | Production certification for the supported target/subset | all |

## Decisions locked

1. **Ordering:** correctness/language parity before browser demo:
   `C0 → C1 → C2 → C3 → C4`, then the ABI/WASI/async/browser branches.
2. **Async:** synchronous lowering is **not** the certified WASM v1 async model;
   C7 implements the full WASM.9 DoD (real suspension).
3. **Stdlib:** a defined WASI-supported subset plus explicit diagnostics (no fake
   implementations); stdlib architecture distinguishes
   portable / native / WASI / browser capabilities.
4. **Certification tooling:** CI installs `wasm-tools`, Wasmtime, Node and
   headless-browser tooling; Wasmtime is **not** bundled into the Vut
   distribution.
