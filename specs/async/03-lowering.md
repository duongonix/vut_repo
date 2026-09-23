# Async Lowering

## 1. Purpose

This document defines how `async fn` and `await` are represented in the compiler
and lowered into state machines.

Async must not remain a loose flag or string in later compiler stages.

---

## 2. Pipeline

```text
Parser
  ↓
AST
  ↓
HIR
  ↓
type checking
  ↓
async lowering
  ↓
MIR
  ↓
optimization
  ↓
codegen
```

Async lowering runs after semantic validation, before MIR code generation.

---

## 3. AST Representation

The AST must distinguish async declarations and await expressions structurally.

Conceptually:

```text
AstFunction
├── name
├── parameters
├── return_type
├── is_async: bool
├── body
└── span
```

```text
AstMethod
├── is_async: bool
├── ...
```

```text
AstExpr
├── Await { value: AstExpr, span }
└── ...
```

Rules:

```text
`is_async` is a typed field, not a string tag
`Await` is a dedicated expression node, not a call
the awaited operand is preserved as a normal expression subtree
the `await` keyword span is preserved
```

---

## 4. HIR Representation

HIR carries the same distinctions after resolution:

```text
HirFunction
├── is_async: bool
├── return_type          # logical result type
└── ...
```

```text
HirExpr
├── Await { value: HirExprId, future_type: TypeId, span }
└── ...
```

The type checker records:

```text
async call expression -> internal future[T]
await expression      -> logical T
```

---

## 5. Lowering Model

Each async function is lowered into:

```text
a future/state object
a start function
a resume function
metadata describing locals that survive suspension
```

Conceptually:

```text
Future<T>
├── state: u32
├── resume: fn(*mut Future<T>, Waker) -> Poll<T>
├── locals that live across await
├── current sub-future
└── completion slot: T
```

The exact physical layout is compiler/runtime internal.

### 5.1 Implementation Note (current phase)

No `await` in this phase can suspend. The implemented lowering therefore
compiles each `async fn` as an ordinary function with its logical return type
and lowers:

```text
await async_call(args)  ->  async_call(args)
```

The awaited call runs to completion on the current thread. This is
observationally equivalent to driving the conceptual state machine to
completion, because the state machine can never return `Pending`.

The state-machine structure described below becomes required as soon as a
suspension source exists. It is the normative model for that extension and must
not be replaced by an ad-hoc mechanism.

---

## 6. State Machine

For:

```vut
async fn example() -> int:
  a = await first()
  b = await second(a)
  b
```

the lowered state machine is conceptually:

```text
State 0
  start first
  suspend

State 1
  receive first result as `a`
  start second(a)
  suspend

State 2
  receive second result as `b`
  complete with `b`
```

States:

```text
state 0        initial / not started
state 1..n     resume points, one per await site
state complete terminal
state dropped  cancelled / destroyed before completion
```

---

## 7. Locals Across `await`

A local must be stored in the state object when its value is live across a
suspension point.

Example:

```vut
async fn example() -> int:
  a = await first()
  b = await second(a)
  b
```

`a` is live across the second `await`, so `a` lives in state storage.

Locals not live across an `await` may remain ordinary MIR locals within a
straight-line segment between suspensions.

The compiler computes liveness per suspension point.

---

## 8. Resume Points

Each `await` introduces a resume point:

```text
await site k
    ↓
state k records what to do when the sub-future completes
    ↓
on resume, extract the logical result and continue
```

A resume point must be able to reconstruct the program state needed after the
suspension. This includes:

```text
the target local for the awaited result
other locals live across this await
the current sub-future slot
source location for diagnostics
```

---

## 9. Completion

When the body reaches its final expression or an explicit `return`:

```text
evaluate result
run cleanup for live state locals
store result in completion slot
mark state complete
wake the awaiting parent
```

An async function returning `void` stores an empty completion value.

---

## 10. Early Return and Error Propagation

`return` and `?` are lowered as transitions to completion after cleanup.

```text
return value:
  compute value
  run cleanup
  complete(value)

`?` on err:
  run cleanup
  complete(err_payload)
```

Cleanup ordering must be deterministic and follow the memory model.

---

## 11. Multiple Awaits

Multiple awaits lower sequentially:

```text
await 1 -> resume -> await 2 -> resume -> ... -> complete
```

No await runs in parallel with another in this phase. There is no implicit
concurrency.

---

## 12. Lowering of `await` in the Caller

An `await` inside an async caller is itself a suspension point of the caller.

```text
caller state:
  start sub-future
  store sub-future in caller state
  return Pending
on wake:
  poll sub-future
  if ready, extract result and continue
  if pending, return Pending
```

Because this phase has no asynchronous I/O, the single-thread executor may
complete the sub-future inline on the first poll. The state-machine structure
must still be produced so the model is correct.

---

## 13. Lazy Start

Calling an async function creates the awaitable value but does not execute the
body until it is driven.

```text
foo()            create future, state 0
await foo()      drive to completion
```

Dropping an unawaited future does not execute the body. It only releases any
resources the future already owns.

---

## 14. Internal Verification

Async lowering must include an internal verification pass that checks:

```text
every await site has a resume state
every state has a valid successor
every live-across-await local is stored
every completion path sets the state to complete
cleanup is inserted on every exit path
no state is unreachable or duplicated
```

A failure here is a compiler bug and must be reported as an internal
diagnostic, not as a user source error.

---

## 15. Lowering Rules

1. AST and HIR represent async structure explicitly.
2. Async lowering runs after type checking.
3. Each async function becomes a state machine.
4. Locals live across an await are stored in the state object.
5. Each await is a resume point.
6. Completion, return, and `?` converge on cleanup plus completion.
7. Await is sequential; there is no implicit concurrency.
8. Unawaited futures are lazy and carry no executed side effects.
9. Internal verification is mandatory.
