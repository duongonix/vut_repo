
# Vut Code Generation

Template-string code generation emits direct typed formatting calls for each
already type-checked segment. It must not parse expressions, use runtime eval,
or box primitive values merely for interpolation.

## 1. Purpose

Codegen converts validated Vut lower IR into native code.

Conceptually:

```text
Typed HIR
↓
MIR
↓
Codegen Backend
↓
Object Code
↓
Linker
↓
Native Executable
```

---

## 2. Responsibilities

Codegen handles:

```text
native types
function ABI lowering
control flow
memory layout
calls
interface dispatch representation
dyn representation integration
runtime calls
object emission
target-specific lowering
```

It must not determine source-level semantic validity.

---

## 3. Backend Abstraction

Define a Vut-owned backend abstraction.

Conceptually:

```text
CodegenBackend
├── initialize_target
├── compile_module
├── emit_object
└── finalize
```

Frontend must not directly emit backend-specific instructions.

---

## 4. Primary Backend Candidate

Preferred initial candidate:

```text
Cranelift
```

Codex may use appropriate Cranelift crates instead of writing machine-code generation manually.

Do not hand-write x86/ARM instruction encoders for MVP.

---

## 5. MIR Boundary

Codegen consumes low-level Vut IR.

It should not consume raw AST.

Bad:

```text
AST -> Cranelift
```

Correct:

```text
AST
↓
HIR
↓
type checked
↓
async lowering (async fn -> state machines)
↓
MIR
↓
Cranelift
```

Async functions are already lowered to state machines and `await` to
suspension/resume points before codegen. Codegen must not implement async
semantics itself.

See:

```text
specs/async/03-lowering.md
specs/async/04-runtime.md
```

---

## 6. Target Independence

MIR should remain largely target-independent.

Codegen translates semantic primitive/layout requirements to target-specific backend types.

---

## 7. Primitive Types

Fixed-width Vut types map predictably:

```text
i8
i16
i32
i64

u8
u16
u32
u64

f32
f64

bool
```

Exact ABI of:

```text
int
float
```

must follow finalized type/ABI specification.

Do not guess unstable ABI promises.

---

## 8. Bool

Internal representation may use efficient native form.

Public/source semantics remain:

```text
true
false
```

No truthiness conversion.

---

## 9. Strings

`str` uses Vut runtime-defined representation.

Do not assume:

```text
char*
```

or NUL termination.

Backend lowering should call/runtime integrate through stable internal runtime APIs.

---

## 10. Lists

Statically typed:

```text
list[T]
```

should retain element type/layout information.

Do not represent all lists as `list[dyn]`.

---

## 11. Data Layout

Named `data` should lower to efficient native layouts.

Possible layout decisions:

```text
field offsets
alignment
size
```

must be centralized.

Do not recompute layout separately in many backend locations.

---

## 11a. Enum Layout

A payload enum is a tagged union:

```text
tag (u8/u16/u32)
payload (max size/alignment of the variants)
```

Codegen emits:

```text
ConstructEnum   store tag + active payload fields
EnumTag         load the active tag
EnumPayload     project a payload field of the active variant
```

Only the active variant's payload owns managed data; drop glue releases only the
active payload. Payload fields are referenced by declaration index, never by a
hashed name.

---

## 11b. Collection Element Ownership

Managed collections do not hardcode their element ownership. Instead the
compiler generates one retain and one release callback per managed element type
and passes their addresses to `list_new` / `map_new`. The runtime stores the
callbacks and invokes them whenever it copies (push/at/set/insert/slice) or
destroys (set-overwrite/remove/clear/drop) an element.

Consequences:

```text
aggregate elements (data, enum, result, array) are stored as pointers
element storage size for aggregates is the target pointer size
trivial elements pass null callbacks and pay no ownership cost
nested managed values inside aggregate elements are retained/released recursively
```

`for` loop bindings retain managed elements and release them at the end of each
iteration, so iterating a collection of managed values is memory-safe.

---

## 12. Layout Engine

Introduce a layout abstraction:

```text
Layout
├── size
├── alignment
└── fields
```

keyed by:

```text
TypeId
```

where useful.

---

## 13. Functions

Each function should lower with:

```text
parameter ABI
return ABI
local storage
basic blocks
calls
```

Function signatures must already be type checked.

---

## 15. Methods

Concrete method source:

```vut
counter.add(10)
```

may lower as a normal native function with explicit receiver argument internally.

Source-level implicit self does not prevent explicit ABI receiver passing.

---

## 16. Interface Calls

Interface values may conceptually lower to:

```text
data/reference pointer
method table pointer
```

or an equivalent efficient representation.

Exact layout is runtime ABI-internal.

---

## 17. Concrete Interface Optimization

If exact concrete type is known, codegen/optimizer should use direct call where possible.

Do not route all method calls through interface dispatch.

---

## 18. `dyn`

`dyn` requires runtime type metadata/value storage.

Only actual dynamic values should pay this overhead.

---

## 19. Optionals

Optional representation may use niche/null optimization.

Example:

```text
ptr[T]?
```

could potentially use null pointer representation internally.

This is implementation-specific and must preserve Vut semantics.

---

## 20. Control Flow

MIR:

```text
basic blocks
branches
jumps
returns
```

maps naturally to backend control flow.

`if`, `for`, `match` high-level syntax should already be lowered appropriately.

---

## 21. Bounds Checks

Safe collection operations that require bounds checking must emit checks unless optimization proves them unnecessary.

Out-of-bounds behavior must follow runtime/std specification. For explicit index
requests (`at`, `set`, `insert`, `remove`) an invalid index must trap through the
shared runtime bounds-panic path (`vut_rt_bounds_panic_v1`); the compiler must
not fabricate a zero/default element. `array` indexing may use a compiler-known
length check (including compile-time rejection of literal out-of-range indexes),
but the runtime failure path is the same shared panic.

---

## 22. Runtime Calls

Runtime functions should have centralized compiler-known declarations.

Examples:

```text
allocation
string operations
list support
panic
dyn support
interface support
```

Do not hard-code runtime symbol names throughout arbitrary codegen modules.

---

## 23. Runtime ABI Version

Compiler must verify compatible runtime ABI.

Mismatch:

```text
E9003
```

---

## 24. External Calls

FFI calls use explicitly supported ABI.

Initial focus:

```text
C
```

Leverage backend calling-convention support.

---

## 25. Object Files

Backend should emit target-native object code.

Link step produces executable/library artifact according to project type.

---

## 26. Linker

Prefer invoking established system/toolchain linker mechanisms or mature Rust tooling.

Do not implement a native linker from scratch.

---

## 27. Linker Errors

Translate linker failures into clear toolchain diagnostics.

Do not dump incomprehensible raw command output when Vut can summarize context, but preserve useful underlying details.

---

## 28. Targets

Codegen architecture should represent:

```text
target triple
CPU architecture
OS
ABI
```

through structured target configuration.

---

## 29. Cross Compilation

Future cross compilation should be enabled by backend architecture.

Do not assume host == target everywhere.

---

## 30. Debug Information

Debug builds may emit source-level debug metadata when backend support is available.

Source spans/HIR origins must be retained sufficiently.

---

## 31. Symbol Names

Internal symbol mangling must avoid collisions.

Mangled names may include:

```text
module
function/type
method
signature identity if required
```

Mangle format is an internal ABI concern unless later stabilized.

---

## 32. Determinism

Given identical compiler inputs/configuration, codegen should aim for deterministic artifacts.

Avoid random symbol ordering.

---

## 33. Parallel Codegen

Independent functions/modules may codegen in parallel where backend/runtime supports it.

Output must remain deterministic.

---

## 34. Tests

Required:

```text
integer arithmetic
floats
bool
functions
calls
methods
data
fields
loops
if
match
lists
strings
interfaces
dyn
runtime calls
linking
multi-module programs
debug/release
multiple targets as support grows
```

---

## 35. Differential Testing

Where possible, compare:

```text
expected semantic execution
optimized native execution
```

for representative programs.

---

## 36. Rules

1. Codegen consumes validated lowered IR.
2. Never codegen directly from parser AST.
3. Backend is abstracted.
4. Prefer proven native backend.
5. Use centralized layout logic.
6. Static values retain efficient static representations.
7. Interface/dyn overhead is localized.
8. Safe bounds behavior is preserved.
9. Runtime ABI is explicit.
10. Target handling is structured and cross-compilation-friendly.
