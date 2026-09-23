# Profile-Guided Optimization Feasibility (M2.6.17)

## 1. Question

Can PGO be built on the Vut/Cranelift 0.135.2 stack, and if so, how?

## 2. Finding

**Cranelift 0.135.2 exposes no PGO primitives.** Concretely:

- There is no branch-probability input to `brif`; block ordering and layout are
  static heuristics.
- There is no profile-driven inlining hook (Cranelift has no inliner at all).
- There is no profile input to register allocation or instruction selection.

Therefore PGO cannot be delegated to the backend; it would be **entirely
Vut-owned**.

## 3. Required architecture (if pursued)

```text
instrumented build
  → MIR instrumentation pass (counters on blocks / call sites)
  → runtime counters (vut_rt_pgo_*)
  → profile file (per function: block counts, call counts, hot/cold)
  → merge tool
  → MIR consumer:
      hot/cold function marking → inlining weights (M2.6.10 cost model)
      branch weights → MIR branch hints / block layout hints
  → rebuild with the profile
```

The only levers Cranelift lets Vut act on are **inlining weights** (Vut's own
MIR inliner) and, weakly, MIR-level branch shaping. Cranelift will not accept
layout weights.

## 4. Decision (M2.6)

**Feasibility + design only; no PGO implementation in M2.6.** Full PGO is not in
the M2.6 DoD. If a later phase pursues it, start with the minimal subset:
instrumentation + hot/cold function marking feeding the M2.6.10 inliner cost
model, behind an off-by-default flag. Profile collection infrastructure, merge
tooling, and branch-weight-driven layout are separate follow-up work.

## 5. Why not now

- High complexity (instrumentation, profile format, consumer, rebuild flow).
- Uncertain benefit given the current corpus (dominated by runtime/allocation).
- No backend support to amplify the profile (no layout/branch weights).

Incremental optimization (M2.6.5–M2.6.14) and target tuning (M2.6.15) offer a
better effort/benefit ratio first.
