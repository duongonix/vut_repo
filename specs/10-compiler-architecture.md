# Vut Compiler Architecture

## 1. Purpose

This document defines the high-level architecture of the Vut compiler.

The compiler is implemented in Rust.

The architecture must prioritize:

- correctness
- modularity
- high-quality diagnostics
- incremental development
- testability
- native performance
- clear separation between language semantics and implementation
- reuse of proven Rust libraries when appropriate

This document does not redefine surface-language semantics.

Language behavior is defined by the other top-level specifications.

---

## 2. Compiler Responsibilities

The Vut compiler is responsible for:

- reading `.vut` source
- lexical analysis
- parsing
- source-span tracking
- module resolution
- name resolution
- semantic validation
- type inference
- type checking
- structural interface checking
- control-flow analysis
- lowering
- optimization
- code generation
- linking/build coordination where applicable
- diagnostics

---

## 3. High-Level Pipeline

The intended pipeline is:

```text
Vut source
    │
    ▼
Source Manager
    │
    ▼
Lexer
    │
    ▼
Parser
    │
    ▼
AST
    │
    ▼
Module Resolver
    │
    ▼
Name / Semantic Resolution
    │
    ▼
HIR
    │
    ▼
Type Inference + Type Checker
    │
    ▼
Interface Checker
    │
    ▼
Control-Flow / Semantic Validation
    │
    ▼
Lowering
    │
    ▼
IR
    │
    ▼
Optimization
    │
    ▼
Code Generation
    │
    ▼
Object / Native Code
    │
    ▼
Link
    │
    ▼
Executable / Library Output
```

Some passes may be combined internally if doing so does not blur responsibilities or reduce diagnostics quality.

---

## 4. Separation of Concerns

Compiler modules must have clear responsibilities.

Avoid implementing:

```text
lexer
parser
type checker
module resolver
codegen
diagnostics
```

inside one large Rust file.

The project may create additional crates/modules/folders as needed.

Modularity is preferred over artificially minimizing file count.

---

## 5. Recommended Workspace Direction

A reasonable architecture may evolve toward:

```text
crates/
├── vut-source/
├── vut-lexer/
├── vut-parser/
├── vut-ast/
├── vut-hir/
├── vut-types/
├── vut-resolver/
├── vut-diagnostics/
├── vut-ir/
├── vut-codegen/
├── vut-compiler/
└── vut-cli/
```

This is a direction rather than a mandatory final crate split.

Do not create unnecessary crates purely for naming symmetry.

Split when a boundary improves:

- maintainability
- compile times
- reuse
- testing
- ownership of responsibilities

---

## 6. Source Manager

The source manager owns source-file information.

It should provide stable identities for files.

Conceptually:

```text
SourceId
SourceFile
SourceSpan
```

Responsibilities include:

- loading source
- storing source text
- mapping byte offsets
- line lookup
- column lookup
- file paths
- source snippets
- package/module origin

Diagnostics should use these stable source identities rather than passing raw filenames everywhere.

---

## 7. Source Spans

A source span should identify a range within a source file.

Conceptually:

```text
Span
├── source_id
├── start
└── end
```

Spans should normally use byte offsets internally.

User-facing line/column calculation should be performed by the source manager/diagnostic layer.

Every syntax and semantic construct that may generate a diagnostic should retain useful spans.

---

## 8. Unicode and Position Tracking

Vut source supports Unicode text.

Internal offsets may use UTF-8 byte positions.

User-facing columns must be calculated consistently.

The compiler must not confuse:

```text
byte offset
Unicode scalar index
display column
```

The exact diagnostic display-column strategy is defined by diagnostics implementation.

---

## 9. Lexer

The lexer transforms source text into tokens.

It handles:

- identifiers
- keywords
- literals
- operators
- punctuation
- comments
- newlines
- indentation-related information
- source spans

Lexer details are defined in:

```text
specs/compiler/lexer.md
```

---

## 10. Indentation Tokens

Because Vut is indentation-based, the lexer/parser pipeline must preserve block indentation reliably.

A common internal model may emit logical tokens such as:

```text
NEWLINE
INDENT
DEDENT
```

This is an implementation detail.

The resulting observable behavior must follow the language syntax specification.

Indentation errors must preserve enough context for precise diagnostics.

---

## 11. Parser

The parser converts tokens into an AST.

It must support error recovery.

A parser must not abort the entire compilation after the first recoverable syntax error.

Parser behavior is defined in:

```text
specs/compiler/parser.md
```

Formal grammar is defined in:

```text
specs/21-grammar.md
```

---

## 12. AST

The AST represents source-level syntax closely.

It should preserve:

- source spans
- identifiers
- syntactic structure
- literals
- declarations
- expressions
- blocks
- imports
- type syntax

The AST should not prematurely encode assumptions belonging to later semantic passes.

Detailed structure is defined in:

```text
specs/compiler/ast.md
```

---

## 13. Error Nodes

Parser recovery may produce explicit invalid/error AST nodes.

These allow later compiler stages to continue safely.

Error nodes should help prevent cascading diagnostics.

Later passes should avoid repeatedly reporting the same root syntax problem.

---

## 14. Module Resolver

The module resolver handles:

- absolute imports
- relative imports
- package roots
- module identity
- source-file mapping
- circular dependencies
- import visibility
- import collisions

Detailed behavior is defined in:

```text
specs/compiler/module-resolver.md
```

Language semantics are defined in:

```text
specs/07-modules-imports.md
```

---

## 15. Module Identity

Modules should use stable internal identities rather than raw string paths throughout the compiler.

Conceptually:

```text
ModuleId
```

A module stores relationships to:

```text
source
parent namespace
imports
symbols
package origin
```

This simplifies dependency graphs and diagnostics.

---

## 16. Dependency Graph

The compiler should build a module dependency graph.

Example:

```text
main
├── app.user
├── app.config
└── math
```

The graph is used for:

- cycle detection
- compilation ordering
- invalidation
- incremental builds

Package resolution from VPM occurs before or alongside source-root construction.

---

## 17. Name Resolution

Name resolution associates identifiers with declarations.

It handles:

- local variables
- constants
- parameters
- functions
- types
- fields
- methods
- interfaces
- enums
- imports
- module aliases

Resolution must respect lexical scope and module privacy.

Resolved identifiers should use stable internal IDs rather than repeatedly relying on strings.

---

## 18. Symbol Identity

Useful internal identities may include:

```text
SymbolId
TypeId
FunctionId
DataId
InterfaceId
ModuleId
```

Exact Rust type names are implementation decisions.

Stable IDs reduce expensive string-based lookup in later passes and simplify caches.

---

## 19. HIR

A High-Level Intermediate Representation may normalize source syntax after name/module resolution.

HIR should represent semantic constructs more directly than the AST.

Possible responsibilities include:

- resolved names
- normalized methods
- normalized control flow
- desugared syntax
- resolved type references
- implicit `self`
- explicit interface relationships at use sites

Details are defined in:

```text
specs/compiler/hir.md
```

---

## 20. Implicit `self` Lowering

Source:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

may internally become conceptually:

```text
Counter.add(self: Counter, amount: int)
```

This internal representation must not change surface syntax or diagnostics.

Diagnostics should still refer to the user's original declaration.

---

## 21. Type Inference

The type system should infer types where specified by the language.

Examples:

```vut
age = 20
name = "Ha"
numbers = @(1, 2, 3)
```

The type checker should establish stable internal types.

Detailed behavior is defined in:

```text
specs/compiler/type-checker.md
```

and:

```text
specs/02-type-system.md
```

---

## 22. Type Interner

The compiler should consider interning canonical types.

Conceptually:

```text
TypeId -> Type
```

This can improve:

- equality checks
- memory use
- caching
- interface compatibility checks

Exact design is implementation-specific.

---

## 23. Type Checking

The type checker validates:

- assignments
- explicit annotations
- function arguments
- function returns
- method calls
- fields
- lists
- optional values
- operators
- conditions
- interface conversion/use
- constants
- control-flow result types

Type errors must carry both expected and found type information where useful.

---

## 24. Error Type

The type system should have an internal error/unknown recovery type.

Once an expression already failed type checking, downstream operations may use an internal error type to prevent cascading diagnostics.

This type is compiler-internal.

It must never be exposed as valid Vut source syntax.

---

## 25. Interface Checker

The interface checker validates structural compatibility.

It checks:

- required methods
- visibility
- parameter types
- return types
- composed interfaces
- conflicting requirements
- interface cycles

Details are defined in:

```text
specs/compiler/interface-checker.md
```

---

## 26. Interface Caching

Structural interface checks may occur frequently.

The compiler may cache compatibility results using keys conceptually similar to:

```text
(concrete_type, interface_type)
```

Caching must account for invalidation in incremental compilation.

---

## 27. Control-Flow Analysis

Compiler semantic analysis should validate:

- unreachable behavior where relevant
- valid `break`
- valid `continue`
- valid `return`
- function return paths
- infinite loops
- match exhaustiveness
- boolean conditions

Advanced analysis may be added incrementally.

---

## 28. Exhaustiveness Analysis

Known finite enums should support `match` exhaustiveness analysis.

The analysis should produce missing-variant diagnostics rather than generic type errors.

Payload pattern analysis can be expanded once payload enum syntax is finalized.

---

## 29. Lowering

After semantic validation, HIR may be lowered into a lower-level IR.

Lowering may transform:

- methods
- interface calls
- ranges
- high-level collections
- `if` expressions
- `match`
- optional/result operations
- `async fn` / `await` into state machines

Async lowering occurs after semantic validation and before MIR construction.
It converts each `async fn` into an awaitable state machine and each `await`
into a suspension/resume point.

See:

```text
specs/async/03-lowering.md
```

Lowering must preserve source-location metadata sufficiently for later diagnostics and debugging information.

---

## 30. Intermediate Representation

The compiler should use an IR suitable for:

- optimization
- code generation
- target-independent analysis

The exact IR strategy is not fixed by the language spec.

Possible approaches include:

- custom MIR/IR
- LLVM IR through a Rust binding
- Cranelift IR
- another proven native backend

The implementation should prefer mature existing libraries instead of implementing a machine-code backend from scratch unless there is a compelling project reason.

---

## 31. Backend Selection

The initial backend should be chosen based on:

- native target support
- optimization quality
- Rust integration
- development complexity
- compile speed
- debugging support
- project goals

Do not build a complete architecture-specific assembler/code generator from scratch for the MVP if a proven backend satisfies the requirements.

---

## 32. Optimization

Optimization is a distinct concern from language semantics.

Possible optimization passes include:

- constant folding
- dead code elimination
- copy elimination
- inlining
- escape analysis
- scalar replacement
- range simplification
- monomorphization-related optimizations
- interface devirtualization when concrete types are known

Optimization details belong to:

```text
specs/compiler/optimization.md
```

---

## 33. Semantics Before Optimization

The compiler must first produce correct unoptimized behavior.

An optimization must never change observable Vut semantics.

Each optimization should have focused tests comparing optimized and unoptimized behavior where possible.

---

## 34. Code Generation

Code generation converts validated IR into target-specific output.

Responsibilities may include:

- native functions
- data layouts
- calling conventions
- interface dispatch
- runtime calls
- constants
- strings
- debug/source metadata

Details are defined in:

```text
specs/compiler/codegen.md
```

---

## 35. Runtime Calls

The compiler may emit calls to a small Vut runtime for operations such as:

- string management
- dynamic values
- collections
- interface support
- panic/internal failure paths
- allocation
- platform abstraction

The runtime should remain as small as practical.

Detailed behavior is defined in:

```text
specs/11-runtime.md
```

---

## 36. Compiler Diagnostics

Compiler stages must emit structured diagnostics rather than directly printing arbitrary strings.

Stages may produce:

```text
Diagnostic
```

objects containing:

```text
code
severity
title
span
labels
notes
help
```

The diagnostic renderer owns terminal formatting.

Detailed design:

```text
specs/compiler/diagnostics.md
```

---

## 37. Diagnostic Ownership

A pass should report an error when it has the best semantic knowledge to explain it.

Examples:

```text
lexer
  invalid literal

parser
  missing colon

resolver
  unknown symbol/module

type checker
  type mismatch

interface checker
  missing required method

control-flow checker
  invalid break
```

Avoid reporting the same root problem independently in several stages.

---

## 38. No Panics for User Errors

Invalid Vut source must not cause compiler panics.

Compiler components must return recoverable errors/diagnostics for expected invalid input.

Rust `panic!` should be reserved for genuine compiler bugs or impossible internal states.

Even then, public builds should convert failures into a controlled internal compiler error where feasible.

---

## 39. Incremental Compilation

The architecture should support future incremental compilation.

The compiler should avoid designs that inherently require reparsing/rechecking the entire dependency graph for every small source change.

Potential cacheable units include:

- source hashes
- token streams
- AST
- HIR
- resolved exports
- type results
- interface results
- IR
- object files

Detailed behavior is defined in:

```text
specs/compiler/incremental-build.md
```

---

## 40. Content Fingerprints

Incremental systems should use content/fingerprint-based invalidation instead of modification timestamps alone.

A module cache key may eventually include:

```text
source hash
compiler version
target
build mode
dependency interface hashes
relevant compiler flags
```

Exact format belongs to incremental-build specifications.

---

## 41. Parallel Compilation

Independent modules may be processed in parallel when dependency relationships permit.

Parallelism must not make diagnostics nondeterministic.

Diagnostic output should preserve deterministic ordering.

---

## 42. Determinism

Given identical:

- source
- dependency lockfile
- compiler version
- target
- relevant flags

the compiler should aim to produce deterministic semantic results.

Reproducible binary output should be pursued where backend/toolchain behavior permits.

---

## 43. Compiler and VPM Boundary

`vpm` is responsible for:

- project management
- dependency resolution
- dependency download
- package version selection
- lockfile management
- project-level orchestration

`vut`/compiler is responsible for:

- source compilation
- module resolution over provided roots
- semantic checking
- native code generation

Do not make the compiler itself implement package network resolution.

---

## 44. Source Roots Provided by VPM

For a project compilation, VPM/compiler orchestration should provide source roots such as:

```text
project src/
dependency math src/
dependency json src/
```

The compiler resolver then treats package names as top-level imported namespaces.

The physical cache path must not become part of source import syntax.

---

## 45. Compiler CLI Boundary

The `vut` CLI intentionally exposes only:

```text
vut run
vut build
```

Compilation internals should be implemented as reusable Rust APIs rather than embedded entirely inside CLI command handlers.

The CLI should remain a thin orchestration layer.

---

## 46. Testing Strategy

Each compiler stage should be independently testable.

Recommended test categories:

```text
lexer tests
parser tests
AST snapshot tests
resolver tests
type tests
interface tests
diagnostic tests
IR tests
codegen tests
end-to-end tests
regression tests
```

Compiler bugs should receive regression tests.

---

## 47. Diagnostic Snapshot Tests

Because diagnostic layout is part of Vut UX, representative diagnostics should have snapshot/golden tests.

Tests should verify:

- error code
- source span
- title
- labels
- notes/help
- plain-text layout

ANSI colors can be tested separately from semantic diagnostic content.

---

## 48. Parser Test Philosophy

Parser tests should cover both valid and invalid Vut.

Invalid syntax is not an edge case.

It is a normal input category because high-quality recovery and diagnostics are required.

---

## 49. Fuzzing

Lexer and parser should be suitable for fuzz testing.

Useful targets include:

- arbitrary UTF-8 input
- indentation combinations
- malformed strings
- malformed numbers
- deeply nested syntax
- parser recovery

The compiler must not crash on arbitrary user source.

---

## 50. Performance Measurement

Performance work should be evidence-driven.

Compiler benchmarks may track:

```text
lex time
parse time
resolution time
type-check time
codegen time
peak memory
incremental rebuild time
```

Runtime benchmarks belong separately to language/runtime performance testing.

---

## 51. Reuse Existing Rust Libraries

The implementation should prefer established Rust crates when they solve a problem well.

Areas where existing libraries should be considered include:

- CLI parsing
- diagnostics support
- semantic version handling
- TOML
- hashing
- graph algorithms
- parallelism
- native backend
- testing utilities

Do not reimplement general-purpose infrastructure merely to reduce dependencies.

Dependencies should still be evaluated for:

- maintenance
- license
- stability
- performance
- unnecessary feature weight

---

## 52. No Monolithic Compiler File

Compiler implementation must not accumulate unrelated logic into one massive module.

For example, avoid:

```text
compiler.rs
  20,000 lines
```

containing:

```text
lexer
parser
types
modules
codegen
diagnostics
```

The implementation is explicitly allowed to create files, modules and crates as needed.

---

## 53. Stable Compiler APIs

Internal APIs may evolve during early development.

However, boundaries between major stages should use clear data structures instead of hidden global state.

Prefer explicit inputs/outputs such as:

```text
parse(source) -> AST + diagnostics

resolve(ast, modules) -> HIR + diagnostics

check(hir) -> typed HIR + diagnostics
```

Exact Rust APIs may differ.

---

## 54. Avoid Global Mutable Compiler State

Compiler context may centralize interners and caches, but arbitrary global mutable state should be avoided.

Explicit compiler/session contexts improve:

- testing
- concurrency
- deterministic builds
- IDE reuse

---

## 55. IDE and Language Server Reuse

The compiler frontend should eventually be reusable by editor tooling.

Lexer, parser, resolver and type checker should not depend directly on process termination or terminal output.

They should return structured results.

This enables:

- language server
- diagnostics while editing
- completion
- hover
- go-to-definition
- rename
- formatter integration

---

## 56. Partial Programs

Editor environments frequently compile incomplete source.

Frontend architecture should support incomplete/error-containing programs without crashing.

Recovery should preserve as much useful structure as possible.

---

## 57. Compiler Architecture Principles

The Vut compiler follows these principles:

1. Compiler is implemented in Rust.
2. Language semantics are separated from implementation details.
3. Major compiler stages have clear responsibilities.
4. Source spans are preserved throughout the frontend.
5. Invalid source produces diagnostics, not crashes.
6. Parser and semantic passes support recovery.
7. Structured diagnostics are shared by CLI and tooling.
8. Internal IDs should replace repeated string lookup where useful.
9. Optimization occurs after semantic correctness.
10. Native backend should reuse proven technology where appropriate.
11. Compiler code should be modular rather than monolithic.
12. VPM handles network/package resolution; compiler handles source compilation.
13. Architecture should permit incremental and parallel compilation.
14. Frontend should eventually be reusable by IDE tooling.
15. Performance improvements should be measured rather than guessed.

---

## 58. Related Specifications

Language behavior:

```text
specs/01-language-syntax.md
specs/02-type-system.md
specs/03-data-model.md
specs/04-functions-methods.md
specs/05-control-flow.md
specs/06-interfaces.md
specs/07-modules-imports.md
specs/08-memory-model.md
specs/09-errors-diagnostics.md
```

Compiler details:

```text
specs/compiler/lexer.md
specs/compiler/parser.md
specs/compiler/ast.md
specs/compiler/hir.md
specs/compiler/type-checker.md
specs/compiler/interface-checker.md
specs/compiler/module-resolver.md
specs/compiler/diagnostics.md
specs/compiler/optimization.md
specs/compiler/codegen.md
specs/compiler/incremental-build.md
```

This document defines the architectural contract that those implementation specifications refine.
