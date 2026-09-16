# Phase 19 — Optimization and Incremental Compilation

## Status

Complete

Release builds run backend-independent MIR constant folding, propagation,
dead-instruction elimination, and dead-branch elimination, building on the
existing last-use move/drop/copy analysis. Aggregates proven local use explicit
stack slots, statically resolved methods remain direct, and Cranelift's `speed`
pipeline performs machine-level simplification, CSE, inlining, scalar and loop
optimizations without duplicating mature backend work.

`vut-incremental` owns schema-versioned BLAKE3 fingerprints, cache keys,
artifact integrity, atomic storage, corruption recovery, and reverse dependency
invalidation. Compiler cache keys include all source/dependency content,
compiler/language version, target, build mode, and runtime ABI. Debug and
release artifacts are isolated. Warm no-change builds reuse the final verified
executable, skipping frontend, MIR, codegen, and linking. VPM places build cache
entries in its global target/mode-aware cache.

The coarse-grained MVP object/executable cache deliberately subsumes separate
serialized AST/HIR/type/MIR caches; changed inputs rebuild safely, while an
unchanged build performs only fingerprint and integrity work.

Measured on `benchmarks/prime_count.vut`: warm no-change build improved from
218.82 ms cold to 8.78 ms warm (24.91x), while release runtime matched the
equivalent optimized Rust workload within measurement noise.
