# SIMD / Vectorization Feasibility (M2.6.16)

## 1. Scope

M2.6.16 enables target SIMD where Cranelift can already use it, and records a
go/no-go decision on an auto-vectorizer. It does **not** implement an
auto-vectorizer.

## 2. What was done (enablement)

- M2.6.15 added opt-in target features: `--target-cpu native` (host detection)
  and `--target-feature avx2`/`+avx2`/`-avx2`, plus `x86-64-v2/v3/v4` presets.
  The default remains the portable baseline.
- With those features enabled, Cranelift can select SIMD for operations it
  already models: bulk `memcpy`/`memset`/`memcmp`, `popcnt`, `lzcnt`, and the
  scalar idioms that lower to them. Verified functionally: a program built with
  `--target-cpu native` produces identical output to the portable build and runs
  (see the CLI smoke check in M2.6.15).

## 3. What Cranelift does not provide

- **No auto-vectorizer.** Cranelift lowers vector IR when it is present but does
  not synthesize vector operations from scalar loops.
- Vut has **no vector type** in the language or MIR, and no vector lowering.

## 4. Go / no-go

**No-go for an auto-vectorizer in M2.6.** A real vectorizer requires all of:

1. vector types in the type system (`i8x16`, `f32x4`, …);
2. legality/aliasing analysis over arrays and slices;
3. a cost model (vector width, trip count, alignment);
4. MIR vector instructions and Cranelift lowering for them.

That is a multi-milestone effort with little near-term coverage: the target
workloads are dominated by ownership/RC/allocation and pointer chasing, not
dense numeric arrays. Recommend a future incremental milestone that starts with
explicit vector types rather than an auto-vectorizer.

## 5. Follow-up

- Automate "SIMD present in generated code" checks by disassembling a small
  vector-friendly kernel per target (`objdump`/`dumpbin`) in CI.
- Revisit explicit vector types once the type system and lowering support them.
