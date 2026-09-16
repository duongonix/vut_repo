
# Vut Compiler Optimization

## 1. Purpose

This document defines optimization architecture for Vut.

Vut targets native performance near systems languages such as C where practical.

Optimization must be considered from the beginning.

However:

```text
optimization must never change language semantics
```

---

## 2. Optimization Philosophy

Correct order:

```text
efficient architecture
↓
efficient IR
↓
avoid unnecessary runtime work
↓
optimization passes
↓
backend optimization
↓
profiling
```

Do not build knowingly inefficient semantics and expect the optimizer to rescue them later.

---

## 3. Optimization Stage

Optimization belongs after semantic correctness has been established.

Conceptually:

```text
AST
↓
HIR
↓
type checking
↓
lowering
↓
MIR / IR
↓
optimization
↓
codegen
```

Do not perform machine-level optimization inside parser/type checker.

---

## 4. MIR

Vut should have a backend-independent middle/lower IR suitable for optimization.

Conceptually:

```text
MIR Function
├── basic blocks
├── instructions
├── values
├── branches
└── calls
```

Exact design belongs to compiler implementation.

---

## 5. Debug vs Release

Debug builds prioritize:

```text
compilation speed
diagnostics
debuggability
```

Release builds enable stronger optimizations.

Do not make debug builds semantically different.

---

## 6. Constant Folding

Examples:

```vut
a = 2 + 3
```

may lower to:

```text
5
```

when evaluation is compile-time safe.

Do not fold operations if doing so changes required runtime error/overflow semantics.

---

## 7. Constant Propagation

Known immutable values may propagate.

ALL-CAPS constants are strong candidates.

---

## 8. Dead Code Elimination

Remove unreachable or unused internal computations when proven safe.

Side effects must be preserved.

---

## 9. Dead Branch Elimination

Example:

```text
if compile-time-known false
```

may remove branch if semantics permit.

Do not misuse lint-level constant conditions as an excuse to alter observable behavior incorrectly.

---

## 10. Copy Elision

Vut's value semantics must not imply unnecessary physical copies.

Compiler may optimize:

```text
move
copy elision
storage reuse
COW
```

as long as observable semantics remain independent-value semantics.

---

## 11. Escape Analysis

Analyze whether values need heap allocation.

Objects not escaping may remain:

```text
stack/register
```

Do not heap-box every `data` value.

---

## 12. Stack Promotion

Heap candidates may be promoted when lifetime/escape proof permits.

---

## 13. Function Inlining

Small/hot functions may inline in release builds.

Avoid excessive inlining that increases code size without benefit.

Use heuristics/backend capabilities.

---

## 14. Devirtualization

Concrete calls should already be direct.

Interface calls may be devirtualized when exact concrete type is known.

---

## 15. Dyn Specialization

When a `dyn` value is statically known during optimization, redundant runtime checks may be removed if semantics remain correct.

Do not make general static code dynamic first and optimize it back later.

---

## 16. Bounds Check Elimination

Safe list accesses may require bounds checks.

Optimizer may remove checks proven redundant.

Example loops over known list length are candidates.

Safe source must not lose memory safety due to speculative elimination.

---

## 17. Common Subexpression Elimination

May reuse repeated pure computations.

Must understand side effects.

Do not CSE calls with unknown effects.

---

## 18. Simplification

Examples:

```text
x + 0 -> x
x * 1 -> x
not not x -> x
```

only where exact type/overflow/NaN semantics allow equivalent transformation.

---

## 19. Loop Optimizations

Potential release optimizations:

```text
loop invariant code motion
bounds-check elimination
strength reduction
vectorization via backend
```

Do not parallelize loops automatically.

---

## 19a. Async State Machines

Optimization may simplify async state machines when semantics are preserved:

```text
inline immediately-ready await chains
remove unused state slots
drop-elide completed segments
devirtualize internal resume calls
```

Async optimization must not:

```text
change suspension order
introduce parallel execution
skip required cleanup
leak or double-drop state values
```

Observable async behavior must remain identical in debug and release builds.

---

## 20. Backend Optimizations

Mature backend should perform machine-level optimizations.

Vut should not reimplement:

```text
register allocation
instruction scheduling
machine peephole optimization
```

unless technically necessary.

---

## 21. Cranelift

Cranelift is the primary candidate for native backend due to:

```text
Rust integration
fast compilation
native code generation
maintained ecosystem
```

Evaluate actual release performance against Vut requirements.

Backend abstraction must allow future alternatives.

---

## 22. LLVM Future

If Vut later requires more aggressive optimization, an LLVM backend may be considered.

Frontend/MIR must not depend on one backend.

---

## 23. Optimization Levels

Potential internal model:

```text
O0
O1
O2
O3
```

Public CLI does not need to expose these immediately.

Current primary distinction:

```text
debug
release
```

---

## 24. Side Effects

IR must model side effects explicitly enough to prevent invalid reordering.

Relevant:

```text
I/O
FFI
memory mutation
panic/failure
global state
```

---

## 25. Purity

Do not assume arbitrary functions are pure.

Future purity annotations may improve optimization but are not currently part of Vut syntax.

---

## 26. Benchmarking

Maintain benchmarks for:

```text
numeric loops
function calls
methods
data
lists
strings
interfaces
dyn
module-heavy programs
```

Measure generated program runtime and compiler time.

---

## 27. Compile-Time Budget

Optimization must balance runtime speed against compiler speed.

Vut should not make ordinary builds extremely slow for negligible gains.

---

## 28. Regression Testing

Optimization bugs require reduced regression tests.

Compare optimized vs unoptimized output/behavior where practical.

---

## 29. Rules

1. Correct semantics come first.
2. Efficient architecture starts before optimization passes.
3. Optimization operates on semantic/lowered IR.
4. Preserve side effects.
5. Exploit static type information.
6. Concrete calls remain direct.
7. Avoid unnecessary heap allocation.
8. Eliminate unnecessary physical copies.
9. Reuse mature backend optimizations.
10. Benchmark every major optimization strategy.



