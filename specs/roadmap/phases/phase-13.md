# Phase 13 — Native Code Generation

## Status

Complete

The Vut-owned backend interface and Cranelift implementation emit real
target-native object files from MIR, validate runtime ABI versions, and invoke the
platform linker to produce an executable. Lowering covers null/integer/float/bool
constants, unary and binary arithmetic, comparisons, SSA locals, branches,
returns, direct static calls, aggregate stack allocation/construction, field
loads using the semantic layout table, runtime calls, static string literal data
(rodata + managed construction), iterator execution, interface/`dyn` dispatch,
and aggregate return ABI (sret). Remaining target-specific ABI classification
details are audited under the Wave 0 correctness gate.

Completed since the last revision: runtime calls for the `bytes` buffer surface,
native construction of the typed `Utf8Error` value, unsigned template formatting
for `u8`/`u16`/`u32`/`u64`, call-result type propagation for `result` returns,
managed-element array cleanup, deterministic native tests (concurrent native
linking is serialized to avoid shared-linker races), and interface/`dyn`
dispatch. Concrete `data`/`enum` values are boxed into reference-counted runtime
boxes with a static vtable whose slot zero is a generated drop thunk; `interface`
method calls load the concrete method address from the vtable and dispatch
indirectly. Concrete-to-interface conversions are inserted at call arguments and
explicit returns. `dyn` boxing and dropping are supported, but `dyn` dispatch
(no method set) and complete target ABI classification remain.

Completed ownership hardening:

- Enum ownership: `manage_value` releases/retains only the active variant's
  managed payload, so dropping a payload enum no longer leaks and nested
  `data`/`array`/`result`/enum chains balance.
- Collection element ownership: the compiler generates per-type retain/release
  callbacks and passes them to `list_new`/`map_new`; the runtime stores and
  invokes them for push/at/set/insert/remove/clear/slice/drop. Managed
  aggregates (`data`, `enum`, `result`, `array`) are stored by pointer, and
  collection literals use the storage size for aggregate elements.
- Iterator ownership: `for` bindings retain managed elements and release them at
  the end of each iteration, so iterating `list(str)`, `list(data)`, and
  `list(enum)` is memory-safe.
- Runtime ABI version 3 records the callback contract.

Still required before Complete: `dyn`/interface value conversion and dynamic
dispatch, complete target ABI classification, and static string data.
