# Phase 12 — Memory Model and MIR

## Status

Complete

`vut-mir` defines backend-independent functions, blocks, values, locals,
terminators, ownership operations, and centralized ABI/type properties. Literal,
local, binding, return, arithmetic, direct function call and method-call lowering
is connected to the compiler. Managed locals use last-use-aware `Move`/`Copy`,
selective `Retain`, moved-value drop elimination, and deterministic normal/return
cleanup. Source conditionals, loops, iterator steps, match decisions, calls,
methods, field access and construction now lower to explicit blocks and
terminators. Cleanup is inserted for normal, return, branch-scope, break,
continue and invalid/error exits.

Managed assignment now drops an initialized destination before overwrite and
resets moved state when storage is reinitialized. Borrowing runtime builtins
balance temporary and retained arguments, managed field projection retains the
field before releasing its aggregate base, and nested aggregate cleanup follows
the centralized field layout recursively.

The architecture refactor introduced dedicated `vut-types`, `vut-interface`
and `vut-memory` boundaries. MIR consumes ownership transfer decisions from the
memory layer rather than embedding them in type checking or code generation.
