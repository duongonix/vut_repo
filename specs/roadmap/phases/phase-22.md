# Phase 22 - Payload Enums and Pattern Matching

## Status

Complete

## Goal

Let enum variants carry named payload fields and provide a full `match` pattern
grammar with exhaustiveness and reachability checking.

## Scope

```text
payload enum declaration and construction
variant payload patterns with bindings
nested, or, range, string, boolean, and integer-literal patterns
guards
exhaustiveness and unreachable-arm checking
tagged-union layout, construction, projection, and drop
```

Out of scope:

```text
generic enums such as Option[T]
recursive enum payloads by value
list/slice patterns (reserved)
optional-value patterns
```

## Tasks

- [x] AST: `EnumVariant` with payload fields; recursive `MatchPattern`; arm guards.
- [x] Parser: variant payload fields; full recursive pattern grammar; guards.
- [x] Resolver: pattern bindings and guard traversal.
- [x] Type checker: variant metadata, construction, pattern checking, exhaustiveness, reachability.
- [x] Diagnostics: E7101-E7112 and W5004.
- [x] MIR: `ConstructEnum`, `EnumTag`, `EnumPayload`, decision-tree match lowering.
- [x] Layout: tagged-union layout and active-payload drop.
- [x] Codegen: tag/payload construction and projection.
- [x] Ownership: managed payload binding, ignored-field drop, no leak.
- [x] Example `examples/enums.vut`.

## Tests

- [x] Parser tests for payload enums and every pattern form.
- [x] Type tests for construction, patterns, exhaustiveness, reachability, recursion, and list patterns.
- [x] MIR tests for enum construction/matching.
- [x] Native E2E for payload enums, patterns, and managed payload.
- [x] `cargo fmt`, `cargo clippy`, and the workspace test suites.

## Decisions

Variant payloads use named `field: Type` fields and are constructed with named
arguments. Patterns bind fields with `field = binding`; a single-field variant
allows the positional shorthand. A bare identifier is a binding unless the
scrutinee enum has a variant of that name. `or` alternatives must bind identical
names. List patterns are parsed but rejected (`E7112`) until their memory
semantics are implemented. Recursive enum payloads by value are rejected
(`E7105`); recursive shapes use an indirect container such as `list[T]`.

## Known Limitations

No generic enums, no list/slice patterns, no optional-value patterns, no
recursive payload indirection.
