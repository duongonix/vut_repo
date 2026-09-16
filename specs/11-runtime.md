# Vut Runtime

## 1. Purpose

This document defines the role and boundaries of the Vut runtime.

Vut is a native compiled language.

The runtime exists only for functionality that cannot reasonably or efficiently be implemented entirely as compile-time transformations or ordinary generated native code.

The runtime must remain:

- small
- predictable
- efficient
- modular
- platform-aware
- independent from compiler frontend logic

Vut must not require a heavyweight virtual machine.

---

## 2. Runtime Philosophy

A compiled Vut program should primarily execute native machine code.

The runtime may provide support for:

- memory allocation
- strings
- collections
- `dyn`
- interface dispatch
- panic/fatal failures
- platform abstraction
- process startup
- standard I/O primitives
- low-level runtime metadata

The runtime must not become a general-purpose VM.

---

## 3. No Virtual Machine

Vut does not execute normal programs through:

```text
bytecode interpreter
JVM-like VM
JavaScript runtime
embedded scripting engine
```

Normal compilation produces native output.

Conceptually:

```text
.vut source
    ↓
Vut compiler
    ↓
native code
    +
small runtime
    ↓
executable
```

---

## 4. Compiler and Runtime Boundary

The compiler handles:

- parsing
- type checking
- interface checking
- semantic validation
- lowering
- optimization
- code generation

The runtime handles only operations requiring runtime support.

The runtime must not perform normal source-level type checking.

---

## 5. Runtime Library

The runtime should be implemented primarily in Rust.

Conceptually:

```text
crates/
└── vut-runtime/
```

The exact project layout may evolve.

The runtime may expose low-level symbols consumed by generated Vut code.

These symbols are compiler implementation details and are not automatically part of the Vut standard library API.

---

## 6. Runtime ABI

Compiler-generated code and the runtime require a stable internal ABI for compatible compiler/runtime versions.

The ABI may define:

- string representation
- list representation
- interface representation
- `dyn` representation
- allocation functions
- runtime metadata
- panic functions

The ABI is internal unless explicitly documented as public.

---

## 7. Memory Allocation

The runtime may provide allocation primitives.

Conceptually:

```text
vut_alloc
vut_realloc
vut_free
```

Exact symbol names are implementation-defined.

Safe Vut source must not call these functions directly.

Normal memory management remains automatic.

---

## 8. Stack and Heap

The compiler decides whether normal values live on:

```text
stack
heap
registers
static storage
```

The runtime may provide heap allocation when necessary.

Source syntax must not depend on these decisions.

---

## 9. Strings

The runtime may provide string support including:

- allocation
- length
- concatenation
- comparison
- Unicode-safe operations
- conversion
- hashing where needed

The exact internal string representation is implementation-defined.

At the language level:

```text
str
```

is a safe value type.

---

## 10. String Representation

The runtime should use UTF-8 as the canonical string encoding unless a later specification intentionally changes this.

Internal representation may conceptually contain:

```text
pointer
length
capacity
```

or an optimized alternative.

Programs must not depend on this physical layout.

---

## 11. Collections

Runtime support may be required for:

```text
list(T)
map(K, V)
bytes
```

Collection implementations should be type-specialized where practical.

The runtime should avoid forcing all collections through dynamic boxed representations.

`bytes` is a contiguous managed buffer (`ptr`/`len`/`capacity`) with a versioned
ABI. The runtime centralizes allocation, growth, slicing, UTF-8 validation, and
release in a dedicated bytes module. UTF-8 validation reports the exact failure
location so the compiler can build a typed `Utf8Error`.

---

## 12. Lists

A list implementation may conceptually contain:

```text
data pointer
length
capacity
```

The compiler may specialize operations based on element type.

List behavior must follow the standard-library collection contract.

---

## 13. Dynamic Values

`dyn` may require runtime type information.

A dynamic value conceptually needs:

```text
runtime type identity
value/storage
cleanup behavior
operation metadata where required
```

Exact representation is implementation-defined.

---

## 14. Dynamic Overhead

`dyn` is explicitly dynamic and may incur costs such as:

- boxing
- runtime type checks
- indirect access
- runtime metadata

Normal statically typed Vut values must not pay this overhead merely because `dyn` exists in the language.

---

## 15. Interface Values

Interface values may require:

```text
value/data pointer
method table
```

Conceptually:

```text
InterfaceValue
├── data
└── vtable
```

The exact layout is compiler/runtime internal.

---

## 16. Interface Method Tables

A method table may contain pointers to concrete implementations required by an interface.

Given:

```vut
interface Animal:
  speak() -> str
```

and:

```vut
fn Dog.speak() -> str:
  "Woof"
```

the compiler/runtime may construct metadata allowing an `Animal` value backed by `Dog` to dispatch to `Dog.speak`.

---

## 17. Static Dispatch Preference

When the concrete type is known:

```vut
dog.speak()
```

the compiler should normally generate a direct call.

Interface dispatch should only be introduced when required by interface-typed behavior.

The existence of interfaces must not force every method call through runtime dispatch.

---

## 18. Devirtualization

The optimizer may convert an interface call into a direct call when the concrete type can be proven.

This optimization must not change observable behavior.

---

## 19. Type Metadata

Runtime type metadata should only be emitted when required.

Possible users include:

- `dyn`
- interface values
- runtime diagnostics
- FFI support
- future reflection features

Normal statically resolved values should not automatically require large runtime metadata tables.

---

## 20. Reflection

General runtime reflection is not part of Vut v1.

The runtime must not maintain expensive reflection metadata for every declaration unless another required feature needs it.

A future reflection system requires a separate specification.

---

## 21. Program Startup

The runtime/compiler toolchain is responsible for connecting platform process startup to the compiled Vut entry point.

Application projects normally use:

```text
src/main.vut
```

The exact generated native entry symbol is implementation-defined.

---

## 22. Program Arguments

Runtime support may expose command-line arguments to the standard library.

The OS-specific representation should be normalized into a Vut API.

Exact API belongs to the standard library specification.

---

## 23. Environment

Runtime/platform support may provide primitives for:

- environment variables
- process information
- current directory
- executable path

User-facing APIs belong to `std`.

---

## 24. Standard I/O

Low-level runtime/platform primitives may support:

```text
stdin
stdout
stderr
```

High-level APIs belong to the standard library.

Compiler diagnostics do not depend on the target program runtime.

---

## 25. Runtime Failures

Certain failures may be unrecoverable.

Examples may include:

- impossible runtime invariant
- allocation failure
- unsafe memory violation
- explicit fatal operation

The runtime should provide a consistent fatal/panic mechanism.

---

## 26. Result Errors vs Runtime Panic

Normal expected application errors should use typed Result-style handling.

Examples:

```text
file not found
network error
invalid user input
parse failure
```

These should not normally become runtime panics.

Runtime panic/fatal behavior is reserved for unrecoverable conditions or violated invariants.

---

## 27. Bounds Checking

Safe collection access must not cause undefined memory access.

If an operation requires bounds checking, the compiler/runtime must preserve memory safety.

The optimizer may remove checks when safety can be proven statically.

---

## 28. Null Safety

Optional values must not become arbitrary null pointers in safe source semantics.

The runtime representation may optimize:

```text
T?
```

using null-pointer optimization where valid.

This representation is not visible to Vut source.

---

## 29. Platform Abstraction

The runtime may contain platform-specific implementations.

Conceptually:

```text
runtime/
├── common/
├── windows/
├── linux/
└── macos/
```

The actual Rust module structure may differ.

Platform-specific logic should remain isolated.

---

## 30. Supported Targets

Target support is a compiler/toolchain concern.

The runtime architecture should avoid unnecessary assumptions that make new targets difficult.

Initial targets may be introduced incrementally.

---

## 31. Runtime Dependencies

The runtime may use proven Rust crates where appropriate.

However, runtime dependencies should be evaluated carefully because they may affect:

- binary size
- startup time
- portability
- compile time
- licensing
- platform support

Do not reimplement complex low-level functionality without reason, but also avoid unnecessary heavyweight dependencies.

---

## 32. Runtime Initialization

Runtime initialization should remain minimal.

Programs that do not require complex runtime features should not pay significant startup overhead.

Avoid large global initialization systems.

---

## 33. Runtime Shutdown

The runtime must correctly release runtime-managed resources according to the memory model.

Normal program termination should not require user-visible cleanup of ordinary Vut memory.

External resources remain governed by their APIs.

---

## 34. Thread Safety

Concurrency is not part of the initial core runtime contract.

Async execution in this phase is single-threaded and uses a minimal executor
defined in:

```text
specs/async/04-runtime.md
```

Runtime components should avoid architectural choices that make future
thread-safe operation impossible.

Detailed threading and synchronization behavior belongs to:

```text
specs/20-concurrency.md
```

---

## 34a. Async Executor

The runtime provides a minimal single-thread executor for `async`/`await`.

```text
async state machine
+
single-thread executor
+
minimal wake/resume mechanism
```

The executor:

```text
runs on the current OS thread
does not spawn worker threads
does not run futures in parallel
is linked only when async is used
```

`async main` is driven to completion by the runtime automatically.

This document's runtime principles are unchanged: the executor is small,
predictable, and internal. No public `Future`, `Poll`, `Waker`, or `Context`
type is exposed to Vut source.

---

## 35. Runtime Version Compatibility

Compiler and runtime versions must be compatible.

The compiler should not silently link against an incompatible runtime ABI.

Compatibility policy is defined in:

```text
specs/23-compatibility-versioning.md
```

---

## 36. Debug Builds

Debug builds may include additional runtime checks such as:

- assertions
- bounds validation
- richer panic information
- debug metadata

Release builds may remove checks only when doing so preserves Vut safety guarantees.

---

## 37. Release Builds

Release builds should prioritize:

- optimized native code
- dead runtime code elimination
- reduced metadata
- devirtualization
- efficient allocation
- minimal runtime overhead

Unused runtime features should not unnecessarily increase binary size.

---

## 38. Runtime Principles

The Vut runtime follows these principles:

1. Vut is native compiled, not VM-based.
2. The runtime remains small.
3. Static code should not pay dynamic overhead.
4. `dyn` may use runtime metadata.
5. Interfaces may use method tables.
6. Concrete calls should prefer direct dispatch.
7. Safe collection operations remain memory safe.
8. Normal application errors use typed errors rather than exceptions.
9. Platform-specific behavior is isolated.
10. Runtime implementation details do not leak into source syntax.
11. Compiler/runtime ABI compatibility is validated.
12. Proven Rust libraries may be reused where appropriate.

This document defines the high-level contract of the Vut runtime.
