# Vut — Agent Rules

These rules are **mandatory** for every implementation, refactor, bug fix, and generated code change in this repository.

## 1. Read the Specs First

Before implementing anything:

* Read the relevant files under `specs/`.
* Treat accepted/chosen behavior in `specs/` as the source of truth.
* Do not invent syntax, semantics, ABI behavior, memory behavior, or architecture that conflicts with the specs.
* If specs are ambiguous or conflicting, stop and report the conflict instead of silently choosing a design.

---

# 2. Never Build Monolithic Files

This is a strict rule.

**DO NOT dump large amounts of logic into one file.**

Every source file must have a clear and focused responsibility.

Bad:

```text
runtime.rs
  parser helpers
  bytes implementation
  UTF-8
  list implementation
  map implementation
  allocation
  diagnostics
  ABI
  2500 lines
```

Good:

```text
runtime/
├── mod.rs
├── bytes/
│   ├── mod.rs
│   ├── buffer.rs
│   ├── access.rs
│   ├── ownership.rs
│   └── utf8.rs
├── list/
├── map/
└── memory/
```

If a feature contains multiple independent responsibilities, create multiple modules.

---

# 3. File Size Rules

Use these limits as architectural guardrails:

```text
0–300 lines     preferred
300–500 lines   review and split when responsibilities can be separated
>500 lines      MUST be reviewed for splitting
>800 lines      MUST be split unless there is a strong documented reason
```

Exceptions may include:

* generated code;
* static tables/data;
* tests where splitting would reduce clarity;
* code that genuinely represents one indivisible responsibility.

Do not bypass these limits by compressing code or placing many unrelated functions together.

---

# 4. Split by Responsibility, Not Arbitrary Size

Do not create meaningless files merely to reduce line count.

Bad:

```text
bytes1.rs
bytes2.rs
bytes3.rs
```

Good:

```text
bytes/
├── mod.rs
├── buffer.rs
├── access.rs
├── conversion.rs
├── ownership.rs
├── cow.rs
└── utf8.rs
```

Every module should answer:

> What single responsibility does this module own?

If there is no clear answer, reconsider the boundary.

---

# 5. Do Not Keep Extending Existing Large Files

The fact that code already exists in a file does **not** mean new code belongs there.

Before adding substantial logic to an existing file:

1. Inspect its current responsibilities.
2. Determine whether the new logic introduces another responsibility.
3. If yes, create a new module.
4. Move related existing logic when necessary.
5. Keep the original file primarily responsible for orchestration or its original purpose.

Do not choose the easiest insertion point at the expense of architecture.

---

# 6. `mod.rs` Is Not a Dumping Ground

`mod.rs` should primarily:

* declare modules;
* re-export APIs;
* contain small shared definitions;
* coordinate closely related modules.

Do not move a monolithic implementation into `mod.rs`.

Bad:

```text
bytes/mod.rs     1800 lines
```

Good:

```text
bytes/mod.rs
bytes/buffer.rs
bytes/access.rs
bytes/ownership.rs
bytes/utf8.rs
```

---

# 7. Respect Crate Boundaries

Do not put functionality into a crate simply because it is convenient.

Compiler stages and subsystems must remain separated.

Examples:

```text
lexer       → tokenization
parser      → syntax parsing
AST         → syntax representation
resolver    → name/module resolution
HIR         → semantic representation
types       → type system
MIR         → lowered executable representation
optimizer   → optimization
codegen     → native code generation
runtime     → runtime primitives
diagnostics → diagnostics
LSP         → editor integration
VPM         → package/project management
```

Do not create hidden coupling between unrelated crates.

---

# 8. Keep Dependency Direction Clean

Avoid circular or backwards architectural dependencies.

Lower-level modules must not depend on higher-level orchestration layers merely to reuse a helper.

If shared functionality is genuinely needed by multiple layers, move it into an appropriate lower-level/shared module.

Do not solve dependency problems with:

* duplicated implementations;
* global mutable state;
* arbitrary callbacks;
* giant utility modules.

---

# 9. No Giant `utils.rs`

Do not create generic dumping grounds such as:

```text
utils.rs
helpers.rs
common.rs
misc.rs
```

unless the responsibility is genuinely cohesive.

Prefer meaningful modules:

```text
path.rs
layout.rs
symbols.rs
utf8.rs
allocation.rs
abi.rs
```

A module name should describe what it owns.

---

# 10. Reuse Existing Infrastructure

Before implementing something new:

1. Search the repository.
2. Find existing abstractions and infrastructure.
3. Reuse or extend them when architecturally appropriate.

Do not create a second implementation of:

* type metadata;
* diagnostics;
* ownership tracking;
* allocation;
* ABI handling;
* symbol resolution;
* target detection;
* runtime representation;
* collection infrastructure.

There should be one canonical implementation for fundamental concepts.

---

# 11. Do Not Patch Around Architecture Problems

Do not fix architectural problems with local hacks.

Avoid:

* special-case branches scattered across unrelated files;
* duplicate state;
* magic values;
* hidden global state;
* stringly-typed internal protocols;
* temporary compatibility paths that become permanent;
* bypassing type/ownership systems.

Fix the correct abstraction instead.

---

# 12. No Undefined Behavior as a Shortcut

Memory safety must not be sacrificed for convenience.

Never knowingly introduce:

* unchecked out-of-bounds memory access;
* use-after-free;
* double free;
* invalid pointer dereference;
* uninitialized reads;
* incorrect ownership transfer;
* invalid ABI layouts.

Unsafe Rust must be minimal, isolated, documented, and justified.

Prefer safe Rust whenever practical.

---

# 13. Ownership Must Be Explicit Internally

For managed Vut values, implementation must correctly account for:

* creation;
* move;
* copy;
* sharing;
* mutation;
* COW;
* retain/release when applicable;
* destruction;
* nested managed values;
* early returns;
* error paths.

Never fix ownership bugs by intentionally leaking memory.

---

# 14. No Silent Semantic Changes

Do not change language behavior merely to make implementation easier.

Especially do not silently change:

* syntax;
* type inference;
* ownership semantics;
* bounds behavior;
* Result behavior;
* Optional behavior;
* ABI;
* visibility;
* module resolution;
* collection semantics.

Any required semantic change must be identified before implementation.

---

# 15. Diagnostics Are Part of the Feature

Invalid Vut programs must fail cleanly.

Do not:

* panic the compiler for normal user errors;
* expose internal Rust errors;
* silently accept invalid programs;
* silently fall back to `dyn`;
* silently substitute default values.

Use the existing diagnostic infrastructure.

---

# 16. Tests Are Required

Every substantial feature or bug fix must include appropriate tests.

Use the relevant levels:

```text
unit tests
type-system tests
parser tests
compile-fail tests
runtime tests
ownership/drop tests
integration tests
E2E tests
regression tests
```

A bug fix should normally include a regression test reproducing the original bug.

Do not remove or weaken existing tests simply to make the suite pass.

---

# 17. Test Failure Must Be Investigated

When a test fails:

Do not immediately modify the test.

Determine whether:

```text
implementation is wrong
spec changed intentionally
test is genuinely obsolete
```

Only update a test when the expected behavior has actually changed.

---

# 18. Refactor Before Adding More Complexity

If implementing a feature exposes an existing architectural problem, fix the boundary first when necessary.

Preferred:

```text
refactor boundary
→ add focused modules
→ implement feature
→ test
```

Not:

```text
add another workaround
→ grow giant file
→ add special cases
→ leave cleanup for later
```

---

# 19. Keep Public APIs Small

Do not expose internal implementation details unnecessarily.

Prefer:

```text
small public API
+
private focused implementation modules
```

Internal helpers should remain internal.

Do not make something public merely to bypass module boundaries.

---

# 20. Avoid Premature Abstraction

Do not create complicated generic frameworks for a single use case.

Prefer the simplest architecture that:

* has clear ownership;
* has clear module boundaries;
* can naturally support known future requirements;
* does not duplicate existing infrastructure.

Abstract when there is a real shared concept.

---

# 21. No Unnecessary Dependencies

Before adding a Rust dependency:

1. Check whether the workspace already has suitable functionality.
2. Check whether the standard library is sufficient.
3. Consider maintenance and binary impact.

Do not reimplement mature functionality when an appropriate established library is clearly better.

Do not add a dependency for trivial functionality.

---

# 22. Do Not Leave Dead Implementations

After replacing an implementation:

* remove obsolete code;
* remove obsolete modules;
* remove unused imports;
* remove dead compatibility paths;
* update callers.

Do not leave old and new implementations running in parallel unless explicitly required.

---

# 23. Formatting and Lints

Modified Rust code must pass:

```text
cargo fmt
cargo clippy
```

Do not silence meaningful warnings without addressing their cause.

Do not add broad lint suppression to hide poor code.

---

# 24. Verify the Whole Workspace

Do not validate only the file or crate being edited.

After substantial changes, run the relevant workspace-level:

```text
build
tests
fmt
clippy
```

When touching memory-sensitive code, also run available:

```text
sanitizers
leak checks
fuzz tests
```

---

# 25. Plan Module Boundaries Before Coding

For every substantial task, determine first:

```text
which crates change
which modules change
which new modules are needed
which existing code should move
where tests belong
```

Then implement.

Do not begin by inserting code into the nearest existing file.

---
# 26. Testing Between Phases

During intermediate phases, **do not write exhaustive tests**.

Only perform lightweight verification to ensure:

* the code compiles;
* the workspace still builds;
* the implemented part basically works;
* no obvious regression, crash, or major error is introduced.

Use this workflow:

```text
implement phase
→ basic test
→ verify build
→ continue to next phase
```

Do not spend significant time building comprehensive test coverage for unfinished features.

**Full testing belongs to the final phase.** After all implementation phases are complete, perform comprehensive testing including unit, integration, regression, ownership/memory, E2E, edge cases, sanitizers, fuzzing, and other relevant tests.


# 27. Final Architecture Review

Before considering a substantial task complete, inspect the resulting diff and ask:

```text
Did any file become too large?

Did any file gain an unrelated responsibility?

Should any new logic be its own module?

Did I duplicate existing infrastructure?

Did I introduce unnecessary coupling?

Are ownership and cleanup correct?

Are errors handled through diagnostics?

Are tests sufficient?

Does the implementation still follow the specs?
```

If any answer reveals an architectural problem, fix it before declaring the task complete.

---

# Core Rule

When choosing between:

```text
fast implementation that increases architectural debt
```

and:

```text
slightly more work with clean module boundaries
```

choose the clean architecture.

Do not optimize for the smallest diff.

Optimize for a compiler codebase that remains understandable, testable, maintainable, and extensible as Vut grows.
