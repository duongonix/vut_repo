# M2.1 — Closure Capture

## Status

Complete. Capturing closures support copy-type, managed, move-only, and receiver
(`self`) captures.

## Goal

Allow lambdas to reference enclosing locals and parameters (`specs/04` §46a),
instead of reporting the retired `E1013`.

```vut
fn main():
  multiplier = 10
  numbers = @[1, 2, 3]
  scaled = numbers.map(x => x * multiplier)
```

## ABI feasibility spike (completed)

Before implementation, a standalone spike hand-built a MIR program that
allocated a heap environment, built a `{code, env}` descriptor, invoked it
indirectly with a hidden leading environment argument, and freed it. It ran
end-to-end through the real codegen and system linker and exited with the
expected value. The spike code and temporary MIR instructions were removed.

## Final design

- `specs/04` §46a, `specs/08` §92: capture by value at closure-creation time.
- A lambda that captures nothing keeps the existing bare code-pointer
  representation and no environment parameter.
- A capturing lambda body receives a hidden leading environment parameter
  (local 0); its ordinary parameters follow.
- A capturing closure value is a **tagged heap block**:
  `[refcount][size][code][drop][captures...]`, tagged with the high bit.
  Non-capturing function values are untagged code pointers, so retain/release
  are no-ops for them.
- `CallIndirect` branches on the tag: the plain path calls a bare code pointer;
  the closure path loads `code` and `environment` and calls
  `code(environment, args...)`.
- Function/callable types are classified `RcClosure`; retaining copies and
  releasing the last owner uses `vut_rt_closure_retain_v1` /
  `vut_rt_closure_release_v1`. The runtime counts live closures so leak checks
  cover them.

## Capture ownership (D2)

- **Copy types** are copied into the environment.
- **Managed types** are moved into the environment at their last use; if the
  binding is still used after the closure, the environment receives a retained
  copy instead.
- **Borrowed values** — a receiver (`self`) or an outer capture — are always
  retained, never moved out of their real owner.
- **Move-only** values may be moved into the environment but cannot also be used
  afterwards; duplication reports `E8010`.
- Each capturing closure with managed captures gets a compiler-generated **drop
  thunk** (`vut_closure_drop_<symbol>`) stored in the block. When the last
  reference is released, the runtime calls the thunk to release captures before
  freeing the block.
- Captured bindings are read-only inside the closure and can be used any number
  of times: each read of a managed capture produces a retained copy for the
  consumer, so the environment keeps its own reference.

## Delivered code

- Resolver: free-variable capture collection (transitive through nested
  lambdas), `E1027` for assignment to a captured binding.
- HIR: `HirFunction.is_closure` and `captures`.
- Type checker: `SemanticResult.closure_captures`.
- MIR: `Instruction::MakeClosure`, `LoadRaw`, `StoreRaw`; per-symbol
  `Program.closure_layouts`; environment prologue and borrowed capture locals;
  move/retain decisions at the creation site.
- Codegen: `MakeClosure`, tagged `CallIndirect` dispatch, `RcClosure`
  retain/release, per-closure drop thunks.
- Runtime: `vut_rt_closure_new_v1`, `vut_rt_closure_retain_v1`,
  `vut_rt_closure_release_v1` (calls the drop thunk), and
  `vut_rt_closure_live_count_v1`.

## Known limitations

- Capture is always heap-allocated; escape analysis and stack environments are
  deferred (`specs/receiver/04`).
- Vutcon/async callbacks and FFI callbacks remain non-capturing.

## Tests

- `lambda_e2e`: capture into a function argument, capture inside `.map`, a
  managed (`str`) capture, a returned `make_adder` closure, a returned closure
  capturing a `str`, a capture still used after the closure, a managed capture
  shared by two closures, and a leak check asserting
  `vut_rt_closure_live_count_v1()` returns to zero.
- `receiver_e2e`: a receiver body capturing an enclosing managed value and a
  receiver body capturing `self`.
- Resolver unit tests: capture recording and `E1027`.
