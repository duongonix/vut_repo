# Async Memory and Ownership

## 1. Purpose

This document defines how Vut's memory model applies across an `await`.

Async lowering must obey:

```text
value semantics
automatic moves
deterministic drop
no tracing GC
no source-level borrow checker
```

The async implementation must not leak state, double-drop, or create
use-after-free.

### 1.1 Implementation Note (current phase)

No `await` in this phase can suspend, so no value actually crosses a native
frame boundary. The implemented lowering evaluates awaited computations inline,
and ownership across an `await` is ordinary sequential ownership.

The state-storage and suspension rules below become required once a real
suspension source exists. They are the normative model for that extension.

---

## 2. Ownership Transfer at Suspension

A native stack frame does not survive a suspension. Therefore any value live
across an `await` must be moved into the async state object before the frame is
suspended.

```text
local live across await
    ↓
move into state slot
    ↓
suspend
```

The stack copy is not used after the move. This is an internal move consistent
with the automatic-move rules of the memory model.

---

## 3. State Storage

The state object is the owner of every value stored for the lifetime of the
suspension.

```text
Future<T>
├── state
├── state slot for `a`
├── state slot for `b`
└── sub-future slot
```

Managed values stored in state retain their ownership metadata:

```text
is_copy
needs_drop
contains_managed
```

State slots are initialized and destroyed exactly once.

---

## 4. Managed Values Across `await`

Examples of managed values that may cross a suspension:

```text
str
bytes
list(T)
map(K, V)
data containing managed fields
interface values
dyn
```

These values are moved into state storage. Any internal reference counts
(shared strings, COW collections) remain balanced: the state slot is one owner.

---

## 5. Resuming

On resume, stored locals are read from state:

```text
resume
    ↓
access state slot `a`
    ↓
continue
```

A value may be moved back to a temporary when its final use occurs in the
resumed segment.

---

## 6. Cleanup on Completion

When an async computation completes normally:

```text
run scope cleanup for all live state locals
drop each initialized state slot exactly once
free the future object
```

Locals already moved out are not dropped again.

---

## 7. Cleanup on Early Return and Error Propagation

`return` and `?` exit the async body early. Cleanup must run before completion.

```text
return value:
  move result into completion slot
  drop remaining live state locals
  complete

`?` err:
  move error into completion slot
  drop remaining live state locals
  complete
```

No live managed value may be skipped.

---

## 8. Cleanup on Cancellation / Drop

An incomplete future may be dropped (for example, an unawaited future, or a
parent future dropped before completion).

Dropping an incomplete future must:

```text
drop the current sub-future recursively
drop every initialized state slot exactly once
mark the future dropped
release the object
```

Cancellation in this phase is not a user-facing API. It is simply correct
destruction of an incomplete future.

---

## 9. Double-Drop Prevention

The state object tracks initialization per slot.

Rules:

```text
a slot is dropped only if initialized
a moved-out slot is marked uninitialized
a dropped slot is marked uninitialized
completion does not re-drop moved-out values
```

---

## 10. Leak Prevention

Every path must release resources:

```text
normal completion
early return
error propagation
dropped-before-first-poll
dropped-while-suspended
panic/fatal during body
```

No state slot may be abandoned.

---

## 11. Use-After-Free Prevention

```text
the state object outlives every live state value
a sub-future outlives the await that drives it
resume only accesses slots valid for its state
```

The compiler must not keep raw pointers into a state object after it is moved,
completed, or dropped.

---

## 12. Ordering

Cleanup order follows normal lexical/deterministic destruction semantics for the
function body. Implementation may choose a deterministic order, but it must be
consistent across debug and release builds.

---

## 13. Value Semantics Preserved

Async must not weaken value semantics.

```text
a value moved into state storage is not observable elsewhere
a value copied before suspension remains independent
COW storage shared before suspension remains balanced
```

---

## 14. Memory Test Requirements

Ownership tests must cover:

```text
managed local live across one await
managed local live across multiple awaits
local moved out before completion
early return with live managed locals
`?` error path with live managed locals
unawaited future dropped
future dropped while suspended
nested async calls
no leak after completion
no double drop
no use-after-free
```

The runtime's live-allocation accounting must return to baseline after each
async computation completes or is dropped.

---

## 15. Memory Rules

1. Values live across suspension are moved into state storage.
2. State slots own their values exactly once.
3. Completion, early return, and `?` all run cleanup.
4. Dropping an incomplete future destroys it safely.
5. No leak, double-drop, or use-after-free.
6. Value semantics remain unchanged by async lowering.
