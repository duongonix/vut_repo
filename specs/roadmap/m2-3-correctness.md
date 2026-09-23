# M2.3 — Correctness Fixes (COW aliasing, range `for`)

Two correctness bugs found after M2.2, fixed as a separate task. No syntax
change and no scheduler/Vutcon change.

## Bug 1 — value semantics / copy-on-write aliasing

### Root cause

A managed collection copied into another binding was shared with a shallow
`Retain` (`specs/08` §37). The runtime mutation operations (`vut_rt_list_push_v1`,
`set`, `insert`, `remove`, `clear`, `sort`, `reverse`, `extend`, `truncate`;
`vut_rt_map_*`; `vut_rt_bytes_*`) mutate storage in place, so mutating the copy
was observable through the original.

```vut
a = @[1, 2, 3]
b = a
b.push(4)   # a must remain @[1, 2, 3]
```

### Fix

Copy-on-write at the mutation site:

- Runtime `vut_rt_list_make_unique_v1` / `vut_rt_map_make_unique_v1` /
  `vut_rt_bytes_make_unique_v1` return the same handle when the reference count
  is 1 (fast path) and otherwise deep-copy the storage (retaining managed
  elements) and release one reference to the shared original.
- MIR `Instruction::MakeUnique` calls the type-appropriate runtime function.
- The MIR builder makes the receiver unique before any mutating builtin:
  - a local receiver is copied, made unique, and stored back;
  - a field receiver is made unique and written back without releasing the old
    value (`FieldStore { release: false }`, because `MakeUnique` already
    consumed it);
  - a subscript receiver (`outer[0].push`) is read as an owned reference, made
    unique, and written back through a queued `ListSet`/`MapSet` after the call;
  - a capture local first retains its borrowed reference, then detaches, and
    becomes owned so cleanup releases it.
- `list.sort_by` (compiler-lowered, in place) uses the same unique receiver.

The unique-owner fast path performs no copy.

### Files

- `crates/vut-runtime/src/abi/list.rs`, `abi/map.rs`, `bytes.rs` — make-unique.
- `crates/vut-runtime/src/abi.rs` — ABI constants and re-exports.
- `crates/vut-mir/src/lowering/ir.rs` — `MakeUnique`, `FieldStore.release`.
- `crates/vut-mir/src/lowering/builder.rs` — receiver detachment + write-back.
- `crates/vut-mir/src/lowering/builder/collections/mod.rs` — `sort_by`.
- `crates/vut-mir/src/optimize.rs`, `lowering/future/values.rs` — scanners.
- `crates/vut-codegen/src/cranelift/instruction.rs` — `MakeUnique`, `release`.

### Tests

`crates/vut-compiler/tests/value_semantics_e2e.rs`: assign-then-mutate for list,
map, and bytes; every list mutation API; `sort_by`; multiple aliases; nested
managed values (`outer[0].push`); function argument; closure capture; field
mutation; unique-owner fast path.

## Bug 2 — `for i in 0..N` range lowering

### Root cause

The type checker accepts `Type::Range` as a `for` iterable, but MIR loop
lowering only handled list/array/variadic iterables. A range fell through to the
generic expression path, so the `RangeExclusive`/`RangeInclusive` binary reached
scalar codegen, which rejects it.

### Fix

`lower_for` lowers a range iterable to a counter loop in MIR:

- both bounds are evaluated exactly once, before the loop;
- the counter is compared against the bound (`<` for exclusive, `<=` for
  inclusive) each iteration;
- `continue` increments the counter before re-testing;
- the loop and optional index bindings receive the counter value;
- empty ranges run zero times.

### Files

- `crates/vut-mir/src/lowering/builder/loops.rs`.

### Tests

`crates/vut-compiler/tests/range_for_e2e.rs`: literal `0..10`; `0..N`;
`start..end`; empty range (`5..5`, `9..2`); inclusive `0..=3`; nested loops;
expression bounds; `break`/`continue`; index binding; bounds evaluated once.

## Verification

`cargo fmt --check`, `cargo clippy --workspace --all-targets`, and
`cargo test --workspace` are clean; `examples_build` passes.
