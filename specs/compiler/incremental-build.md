
# Vut Incremental Build

## 1. Purpose

Incremental compilation avoids repeating compiler work when source or dependencies have not changed.

Vut must be architected for incremental compilation from the beginning even if the first implementation initially recompiles more than the final system.

Do not design compiler state that makes future incremental support require a complete rewrite.

---

## 2. Goal

On source change:

```text
recompute only affected work
```

rather than:

```text
recompile entire world unconditionally
```

when correctness allows reuse.

---

## 3. Incremental Layers

Potential reusable stages:

```text
source hashing
lexing
parsing
module resolution
HIR
type checking
interface checking
MIR
codegen
dependency object artifacts
```

Not all need caching in MVP.

Architecture must allow staged adoption.

---

## 4. Source Fingerprint

Each source file should have a content fingerprint.

Preferred high-performance candidate:

```text
BLAKE3
```

Use a proven crate.

Do not implement a custom hash.

---

## 5. File Metadata Is Not Enough

Do not rely only on:

```text
mtime
file size
```

for correctness-critical reuse.

Metadata can be used as a fast precheck, but content fingerprinting should verify when needed.

---

## 6. Compilation Inputs

Cache keys must account for relevant inputs.

Conceptually:

```text
source hash
compiler version
language compatibility version
target
build mode
feature/configuration
dependency fingerprints
runtime ABI
cache schema
```

---

## 7. Source Dependency Graph

Module resolver provides:

```text
ModuleGraph
```

Example:

```text
A -> B
A -> C
B -> D
```

If `D` changes, affected reverse dependents may include:

```text
B
A
```

---

## 8. Public vs Private Changes

Future incremental optimization should distinguish:

```text
implementation-only change
public semantic interface change
```

Example:

Changing private function body in `math` should not necessarily force type-checking every dependent module if exported signatures remain unchanged.

---

## 9. Interface Fingerprint

Modules/packages may have a semantic/public fingerprint derived from:

```text
public symbols
types
function signatures
data shape
interface requirements
enum variants
```

Dependents can reuse semantic results when public interface is unchanged.

---

## 10. AST Cache

Parsed AST may be cached using:

```text
source hash
parser/cache format version
```

But serialized AST cache is optional.

In-memory incremental reuse may be simpler initially.

---

## 11. HIR Cache

HIR reuse requires dependencies such as name/module resolution state to remain compatible.

Do not reuse HIR solely because source text is unchanged if imported public semantics changed.

---

## 12. Type Check Cache

Type-check result validity depends on:

```text
module HIR
referenced symbol/type signatures
interface shapes
```

Explicit dependency tracking is preferable to broad global invalidation.

---

## 13. Interface Cache

Cache structural compatibility:

```text
(TypeId, InterfaceId)
```

but invalidate if either semantic shape changes.

---

## 14. MIR Cache

MIR reuse depends on typed HIR and relevant optimization configuration.

---

## 15. Object Cache

Generated object files are strong incremental-build candidates.

Key by:

```text
MIR/semantic fingerprint
target
optimization mode
backend version/config
runtime ABI
```

---

## 16. Package Build Cache

Dependencies from VPM are immutable by version/revision.

This makes package build results highly cacheable.

Conceptually:

```text
package source checksum
+
compiler
+
target
+
build mode
=
package build cache identity
```

---

## 17. Global VPM Build Cache

VPM may store dependency compilation cache under:

```text
~/.vpm/cache/build/
```

Projects using identical package/version/target/compiler may reuse compiled dependency artifacts where safe.

---

## 18. Local Project Cache

Project-specific intermediate results may live under:

```text
build/
```

or compiler-managed metadata directories according to final build layout.

Do not pollute source folders.

---

## 19. Cache Schema Version

Internal cache format must be versioned.

If incompatible:

```text
discard
rebuild
```

Do not attempt unsafe interpretation.

---

## 20. Cache Corruption

Cache is never source of truth.

On corruption:

```text
invalidate entry
recompute
```

where possible.

If recomputation fails, report actual compilation failure, not stale cache behavior.

---

## 21. Atomic Writes

Cache entries must not become visible until completely written.

Use temporary file/directory + atomic rename where practical.

---

## 22. Parallel Builds

Module graph may permit parallel processing of independent nodes.

Example:

```text
A depends on B and C
B and C independent
```

Compile:

```text
B || C
```

then A.

---

## 23. Determinism

Parallel incremental builds must produce stable semantic results and diagnostics.

Do not let task completion order change output.

---

## 24. Work Scheduling

Potential CPU parallelism:

```text
rayon
```

may be used.

Do not parallelize tiny tasks where scheduling overhead exceeds benefit.

---

## 25. Build Sessions

Use explicit:

```text
CompilerSession
```

or equivalent.

A session owns:

```text
source database
module graph
semantic tables
cache context
diagnostics
target/configuration
```

No process-global compiler database.

---

## 26. Query Architecture

Compiler may eventually adopt a query-like architecture.

Conceptually:

```text
parse(SourceId)
resolve(ModuleId)
type_check(ItemId)
lower(FunctionId)
codegen(ModuleId)
```

Each query tracks inputs/dependencies.

Do not introduce a complex query engine prematurely if simpler architecture suffices, but keep APIs compatible with dependency-oriented computation.

---

## 27. Salsa-Like Systems

A Rust incremental computation framework may be evaluated if it provides clear value.

Do not adopt it merely because another compiler uses it.

Assess:

```text
performance
architecture fit
maintenance
diagnostic integration
parallelism
memory cost
```

---

## 28. Fine-Grained vs Coarse-Grained

Start with sensible granularity.

Example initial approach:

```text
per module
```

instead of immediately building per-expression dependency tracking.

Increase granularity when benchmarks justify it.

---

## 29. Cold Build

Incremental architecture must not make clean builds unreasonably slow.

Benchmark:

```text
cold build
warm no-change build
single-file edit
public API edit
dependency edit
```

---

## 30. No-Change Build

Eventually:

```text
vpm build
```

after no source/config changes should perform minimal work.

It should not:

```text
reparse everything
rehash huge unchanged dependency trees unnecessarily
recompile all dependencies
relink needlessly where avoidable
```

---

## 31. Relinking

If final executable must relink due to one changed object, only required object/codegen work should rerun before linking.

Future linker-level incremental capabilities may be leveraged where available.

---

## 32. Dependency Changes

Changing `vpm.lock` or resolved package contents must invalidate relevant dependency/module results.

---

## 33. Compiler Upgrade

Compiler upgrades may invalidate caches.

Correctness is more important than cache reuse.

---

## 34. Target Changes

Switching:

```text
x86_64
aarch64
```

must use distinct codegen/build cache identities.

Frontend parse cache may remain reusable where independent of target.

---

## 35. Debug/Release

Debug and release object/optimization caches must remain separate.

Shared frontend results may be reused if configuration does not affect them.

---

## 36. Diagnostics Cache

Caching failed semantic results is possible but must not preserve stale source line/span information incorrectly.

Initial implementation may simply recompute invalid/error modules.

---

## 37. Observability

Internal tracing should make incremental behavior measurable.

Useful metrics:

```text
files reused
files parsed
modules type-checked
functions code-generated
cache hits
cache misses
time per phase
```

Use tracing/benchmark infrastructure rather than production debug prints.

---

## 38. Tests

Required scenarios:

```text
clean build
no-change rebuild
single function body edit
public function signature edit
private declaration edit
new import
removed import
dependency version change
target change
debug → release
cache corruption
compiler cache-version change
parallel independent modules
```

---

## 39. Performance Benchmarks

Create representative multi-module fixtures.

Track:

```text
cold compile time
warm compile time
incremental single-edit time
memory usage
cache size
```

---

## 40. Rules

1. Incremental support is an architectural requirement.
2. Cache is never source of truth.
3. Track explicit dependencies.
4. Module graph drives invalidation.
5. Public semantic fingerprints may reduce invalidation.
6. Immutable packages should be aggressively reusable.
7. Cache keys include all correctness-critical configuration.
8. Cache formats are versioned.
9. Corrupt caches are discarded safely.
10. Parallelism must remain deterministic.
11. Start coarse-grained, improve based on measurement.
12. Do not sacrifice cold-build performance for excessive incremental complexity.
13. No global mutable compiler database.
14. Benchmark incremental behavior continuously.
