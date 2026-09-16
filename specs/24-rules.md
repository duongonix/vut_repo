# Vut Development Rules

## 1. Purpose

This document defines mandatory engineering rules for implementing Vut.

These rules apply to:

* Vut compiler
* Vut runtime
* VPM
* standard library
* formatter
* linter
* testing infrastructure
* package resolver
* CLI tools
* future language tooling

This file is normative.

Codex MUST follow these rules while implementing Vut.

The goal is not merely to make Vut work.

The goal is to build Vut correctly from the beginning with:

* high performance
* clean architecture
* clear module boundaries
* maintainability
* scalability
* testability
* low technical debt
* predictable behavior

The implementation philosophy is:

> Write each part correctly when it is introduced.

Do NOT follow:

```text
make it work first
optimize it later
rewrite architecture later
```

for architectural or performance-critical decisions that can reasonably be designed correctly from the beginning.

---

# 2. Core Engineering Rule

Every implementation must satisfy four requirements:

```text
correct
clean
modular
efficient
```

Code that merely produces the expected output is not automatically acceptable.

Before considering a feature complete, Codex must consider:

```text
Is the architecture correct?

Is this code in the correct module?

Can this component grow without becoming a giant file?

Are unnecessary allocations or copies being introduced?

Is there already a proven Rust crate that solves this better?

Can this implementation be tested independently?

Will future compiler phases be able to reuse this component?

Does this follow the Vut specifications?
```

---

# 3. Specifications Are the Source of Truth

All implementation must follow:

```text
specs/
```

The specifications define Vut behavior.

Code must not silently redefine the language.

If implementation and specification disagree:

```text
specification wins
```

unless the specification is explicitly changed first.

Codex must not invent language behavior merely because another language uses it.

---

# 4. Do Not Invent Missing Language Features

When a feature is marked:

```text
TBD
deferred
future
not finalized
not locked
```

Codex must NOT invent its syntax or semantics.

Examples include currently deferred areas such as:

```text
thread model
channel syntax
atomic API
capturing closures
generic function syntax
payload enum syntax
static method syntax
advanced pattern matching
advanced FFI syntax
```

`async fn` and `await` are no longer deferred for the single-thread foundation;
they are specified by `specs/async/`. Do not extend that foundation into
threads, channels, or parallel execution without a separate specification.

If implementation reaches one of these areas, leave an appropriate internal extension point rather than inventing the public language design.

---

# 5. Mandatory Modular Architecture

The project MUST be divided into clear modules/crates.

Do not place unrelated compiler responsibilities into one module.

Conceptually:

```text
source
  ↓
lexer
  ↓
parser
  ↓
AST
  ↓
HIR
  ↓
name resolution
  ↓
type checking
  ↓
interface checking
  ↓
lowering
  ↓
optimization
  ↓
code generation
  ↓
linking
```

Each stage must have a clear responsibility.

---

# 6. Recommended Workspace Architecture

The exact structure may evolve, but the architecture should resemble:

```text
vut/
├── Cargo.toml
├── Cargo.lock
│
├── crates/
│   ├── vut-source/
│   ├── vut-lexer/
│   ├── vut-parser/
│   ├── vut-ast/
│   ├── vut-hir/
│   ├── vut-resolver/
│   ├── vut-types/
│   ├── vut-interface/
│   ├── vut-diagnostics/
│   ├── vut-middle/
│   ├── vut-codegen/
│   ├── vut-runtime/
│   ├── vut-compiler/
│   ├── vut-cli/
│   │
│   ├── vpm-core/
│   └── vpm-cli/
│
├── std/
├── tests/
├── benches/
└── specs/
```

This is an architectural direction, not a requirement that every crate must exist immediately.

Do NOT create empty crates merely to match this tree.

Create a crate/module when its responsibility actually exists.

---

# 7. Crate Boundaries Must Have Meaning

Do not create excessive tiny crates.

A crate boundary should exist when it provides a meaningful architectural boundary such as:

```text
compiler frontend
diagnostics
runtime
code generation
package manager
CLI
```

Within a crate, use Rust modules to divide smaller responsibilities.

Bad:

```text
crates/
├── token/
├── span/
├── string/
├── parser-expression/
├── parser-statement/
├── parser-data/
└── parser-loop/
```

This creates unnecessary dependency complexity.

Prefer:

```text
vut-parser/
└── src/
    ├── lib.rs
    ├── parser/
    │   ├── expression.rs
    │   ├── statement.rs
    │   ├── declaration.rs
    │   └── control_flow.rs
    └── recovery/
        └── mod.rs
```

---

# 8. No Giant Files

Codex MUST NOT put excessive logic into one source file.

Files should represent one coherent responsibility.

Bad:

```text
parser.rs
```

containing:

```text
lexer integration
expressions
statements
imports
data
interfaces
loops
errors
recovery
AST lowering
```

thousands of lines long.

Prefer:

```text
parser/
├── mod.rs
├── expression.rs
├── statement.rs
├── declaration.rs
├── data.rs
├── interface.rs
├── import.rs
├── control_flow.rs
└── recovery.rs
```

---

# 9. File Size Is a Signal, Not the Only Rule

There is no arbitrary absolute line limit.

However, when a file becomes difficult to understand because it contains multiple responsibilities, split it.

A file approaching several hundred lines should trigger a review of whether responsibilities can be separated.

Do not split files mechanically only to reduce line count.

The goal is:

```text
high cohesion
low coupling
clear responsibility
```

---

# 10. Functions Must Remain Focused

Avoid functions that perform many compiler stages at once.

Bad:

```text
parse_resolve_typecheck_and_codegen()
```

Prefer:

```text
parse()
resolve()
type_check()
lower()
codegen()
```

Each function should perform a focused operation.

---

# 11. Avoid Deeply Nested Logic

Avoid excessive nesting.

Bad conceptual structure:

```text
if ...
  for ...
    if ...
      match ...
        if ...
          ...
```

Prefer:

* early returns
* helper functions
* focused modules
* explicit state transitions
* appropriate enums

Complex compiler logic must remain readable.

---

# 12. Prefer Explicit Compiler Data Structures

Compiler architecture should use explicit representations.

Examples:

```text
Token
Span
SourceId
AstNode
HirNode
TypeId
SymbolId
ModuleId
Diagnostic
PackageId
```

Do not pass loosely structured strings/maps everywhere.

Strong internal types reduce compiler bugs.

---

# 13. IDs Instead of Repeated Large Copies

Compiler structures should use compact IDs/indices where appropriate.

Conceptually:

```text
TypeId
SymbolId
ModuleId
SourceId
NodeId
```

instead of repeatedly cloning large objects.

Example:

```text
SymbolId -> SymbolTable
TypeId   -> TypeArena
```

This improves:

* memory usage
* lookup performance
* ownership simplicity
* cache locality

---

# 14. Avoid Unnecessary Cloning

Do not use:

```rust
.clone()
```

as the default solution to ownership problems.

Before cloning, determine whether the code should instead use:

```text
borrow
reference
ID
arena
Arc
move
Cow
```

Cloning small cheap values is acceptable.

Cloning large:

```text
AST
HIR
strings
source buffers
type structures
module structures
```

repeatedly is not.

---

# 15. Avoid Unnecessary Allocation

Compiler hot paths should avoid unnecessary heap allocations.

Particularly:

```text
lexer
parser
symbol lookup
type checking
module resolution
HIR traversal
codegen
```

Avoid creating temporary:

```text
String
Vec
HashMap
Box
```

values when borrowed data or reusable buffers are sufficient.

---

# 16. Do Not Optimize Blindly

Performance-first does NOT mean writing obscure code without evidence.

The rule is:

```text
choose efficient architecture from the beginning
+
measure important hot paths
+
avoid known unnecessary overhead
```

Not:

```text
micro-optimize every line
```

Readability must remain high unless profiling demonstrates that a more complex implementation is justified.

---

# 17. Performance Is an Architectural Requirement

Performance must be considered while designing each subsystem.

Do NOT intentionally choose a clearly inefficient architecture with the plan:

```text
we will optimize it later
```

Examples of unacceptable temporary architecture:

```text
reparse every source file repeatedly
clone AST between every compiler phase
store every compiler object as dynamic maps
perform linear symbol lookup everywhere
re-read unchanged files every build
re-download dependencies every build
serialize/deserialize IR between every internal phase without need
```

---

# 18. Correct Architecture Before Micro-Optimization

Priority:

```text
1. correct algorithms
2. correct data structures
3. correct ownership model
4. correct module architecture
5. avoid unnecessary work
6. benchmark
7. micro-optimize proven hotspots
```

Do not reverse this order.

---

# 19. Complexity Must Be Considered

When choosing algorithms, consider:

```text
time complexity
memory complexity
expected project size
frequency of operation
```

Examples:

Symbol lookup should not repeatedly scan all symbols.

Module lookup should not repeatedly traverse the entire project.

Package version selection should not repeatedly query the provider when cached metadata is valid.

---

# 20. Incremental-Friendly Architecture

Even before full incremental compilation exists, compiler architecture should allow it later.

Compiler stages should make dependencies explicit.

Avoid hidden global state.

Useful conceptual dependencies:

```text
source file
   ↓
tokens
   ↓
AST
   ↓
HIR
   ↓
semantic results
```

Changes should eventually invalidate only affected downstream results.

---

# 21. Avoid Global Mutable State

Do not use process-global mutable state for normal compiler data.

Prefer explicit contexts:

```text
CompilerSession
SourceDatabase
TypeContext
ModuleGraph
DiagnosticSink
```

This improves:

* testing
* concurrency
* incremental compilation
* deterministic builds

---

# 22. Determinism Is Mandatory

Given identical:

```text
source
dependencies
compiler version
target
configuration
```

the compiler should produce deterministic semantic results.

Do not allow hash-map iteration order or thread scheduling to change:

```text
diagnostic order
module order
symbol order
lockfile order
```

Use deterministic sorting where observable ordering matters.

---

# 23. Reuse Proven Rust Libraries

Codex is explicitly ALLOWED and ENCOURAGED to install Rust crates required for implementation.

Do not avoid dependencies merely to keep dependency count low.

If a mature, maintained and performant crate already solves a general problem well, prefer it over reimplementing that problem from scratch.

---

# 24. Do Not Reinvent General Infrastructure

Do NOT manually reimplement mature general-purpose infrastructure without a Vut-specific reason.

Examples:

```text
SemVer parser
TOML parser
HTTP client
TLS
cryptographic hashing
CLI argument parser
Unicode identifier processing
parallel work scheduler
temporary directory handling
URL parsing
compression/archive formats
terminal color handling
```

Use proven crates.

---

# 25. Dependency Selection Criteria

Before adding a dependency, evaluate:

```text
performance
maintenance status
API quality
correctness
security history
platform support
dependency weight
license
ecosystem adoption
MSRV requirements
```

Do not automatically choose the crate with the most GitHub stars.

Choose the crate that best fits Vut.

---

# 26. Dependencies Must Not Dictate Vut Design

External libraries are implementation tools.

They must not dictate Vut's public language syntax.

Example:

If the parser library prefers a certain syntax, but Vut specification defines another syntax:

```text
Vut specification wins
```

Adapt the parser implementation.

Do not change Vut to fit the crate.

---

# 27. Recommended Dependency Categories

The following crates are recommended starting points.

Codex may select better alternatives if justified by:

```text
performance
maintenance
compatibility
architecture
```

Do not blindly install every crate listed here.

Install only what the current implementation actually needs.

---

# 28. CLI

Recommended:

```text
clap
```

Use for:

```text
vut
vpm
```

Responsibilities:

```text
argument parsing
flags
subcommands
help generation
validation
```

Do not manually implement a CLI parser.

---

# 29. Lexer

Strong candidates:

```text
logos
```

or a carefully implemented custom lexer if Vut's indentation-sensitive syntax makes that architecture demonstrably simpler/faster.

`logos` is preferred when it cleanly fits the tokenization model.

Do not introduce a heavyweight parser framework merely to tokenize simple syntax.

---

# 30. Parser

Parser choice must prioritize:

```text
excellent error recovery
precise spans
performance
maintainability
indentation-sensitive syntax
future language evolution
```

Candidates worth evaluating:

```text
chumsky
winnow
lalrpop
custom recursive descent / Pratt parser
```

For Vut, a well-structured recursive-descent parser plus Pratt expression parser may be preferable if it provides better control over diagnostics and recovery.

Do NOT write an unstructured ad-hoc parser.

If custom parsing is used, it must still be modular and heavily tested.

---

# 31. Syntax Tree

If Vut requires a lossless syntax tree for:

```text
formatter
IDE
LSP
refactoring
incremental parsing
```

evaluate:

```text
rowan
```

Do not force Rowan into the semantic AST/HIR if a simpler representation is more appropriate.

Syntax tree and semantic IR may be different structures.

---

# 32. Diagnostics

Recommended candidates:

```text
codespan-reporting
ariadne
annotate-snippets
```

Select the one that best supports Vut's required diagnostic model:

```text
source file
line
column
syntax-highlighted snippet
precise spans
multiple labels
notes
help
terminal color
```

The internal Vut diagnostic representation should remain independent enough that rendering libraries can be changed later.

Do not scatter renderer-specific diagnostic types throughout the compiler.

---

# 33. TOML

Recommended:

```text
toml
toml_edit
serde
```

Use:

```text
toml
```

for straightforward parsing/serialization.

Use:

```text
toml_edit
```

where VPM must modify manifests while preserving formatting/comments.

Use:

```text
serde
```

for typed serialization/deserialization where appropriate.

Do not implement TOML parsing manually.

---

# 34. Semantic Versioning

Use:

```text
semver
```

for:

```text
package versions
prerelease ordering
version validation
future version constraints
```

Do NOT implement SemVer parsing or comparison manually.

---

# 35. HTTP

Recommended:

```text
reqwest
```

for VPM provider communication unless a lighter alternative is proven better for the final architecture.

Use it for:

```text
GitHub API
GitLab API
package downloads
metadata requests
```

Do not implement HTTP or TLS manually.

---

# 36. URL Handling

Recommended:

```text
url
```

Use proper URL parsing.

Do not concatenate URLs using fragile string operations.

---

# 37. Cryptographic Hashing

Recommended candidates:

```text
blake3
sha2
```

Choose according to package-format/checksum compatibility requirements.

For internal high-performance content hashing, `blake3` is a strong candidate.

For interoperability requiring SHA-2, use `sha2`.

Do not implement cryptographic hashing manually.

---

# 38. Temporary Files

Recommended:

```text
tempfile
```

Use for:

```text
downloads
tests
atomic temporary output
package extraction
```

Do not manually create insecure predictable temporary filenames.

---

# 39. Filesystem Traversal

Recommended:

```text
walkdir
```

or:

```text
ignore
```

Use `ignore` when Git-style ignore handling or efficient parallel project traversal is useful.

Do not repeatedly implement recursive filesystem walking.

---

# 40. Parallelism

Recommended:

```text
rayon
```

for CPU-bound independent compiler work where parallelism is beneficial.

Potential use:

```text
independent module processing
parallel package hashing
selected compiler analyses
```

Do not parallelize everything automatically.

Parallelism must preserve deterministic behavior.

---

# 41. Async Runtime

Do NOT add a multi-threaded async runtime to the compiler/runtime merely because
one exists.

The Vut language runtime defines only the minimal single-thread executor needed
for `async fn`/`await`:

```text
specs/async/04-runtime.md
```

It must not introduce worker threads, thread pools, or parallel execution.

For VPM network operations, evaluate whether:

```text
tokio
```

is justified by actual concurrent I/O requirements.

If synchronous networking is sufficient and simpler, avoid unnecessary runtime complexity.

Multi-threaded language concurrency remains deferred by `20-concurrency.md`.

---

# 42. Graph Algorithms

Potential candidates:

```text
petgraph
```

for:

```text
module dependency graph
package dependency graph
cycle detection
topological ordering
```

Use it if it simplifies implementation without introducing unnecessary overhead.

For simple specialized graphs, a compact custom graph using IDs and vectors may be faster and clearer.

Evaluate rather than blindly installing.

---

# 43. Fast Hash Maps

Start with standard library collections unless profiling or workload characteristics justify alternatives.

Potential candidates:

```text
rustc-hash
hashbrown
indexmap
```

Use cases:

```text
rustc-hash
  compiler-internal non-adversarial hash maps

hashbrown
  advanced hash-table control

indexmap
  deterministic insertion ordering
```

Do not replace every `HashMap` automatically.

Security-sensitive untrusted-input scenarios must consider hash-flooding resistance.

---

# 44. Arena Allocation

Compiler AST/HIR/type structures may benefit from arena allocation.

Potential candidates:

```text
bumpalo
typed-arena
la-arena
```

Evaluate according to:

```text
lifetime model
stable IDs
allocation patterns
incremental compilation requirements
```

For semantic structures requiring stable compact IDs, an indexed arena approach may be preferable.

---

# 45. Compact Strings / Interning

Compiler workloads contain many repeated identifiers.

Evaluate string interning.

Potential crates:

```text
lasso
string-interner
```

Potential targets:

```text
identifiers
module names
field names
method names
type names
```

Do not allocate duplicate owned `String` values everywhere.

Measure memory/performance impact.

---

# 46. Error Types

For application/tool boundaries:

```text
thiserror
```

is recommended for structured library errors.

`anyhow` may be used at high-level CLI/application boundaries where rich typed matching is unnecessary.

Compiler user diagnostics should NOT simply become `anyhow::Error`.

Keep:

```text
internal Rust errors
```

separate from:

```text
Vut source diagnostics
```

---

# 47. Logging and Tracing

Recommended:

```text
tracing
tracing-subscriber
```

for internal:

```text
compiler tracing
VPM diagnostics
performance instrumentation
debug logging
```

Do not use uncontrolled `println!` debugging throughout production compiler code.

---

# 48. Benchmarking

Recommended:

```text
criterion
```

or stable Rust benchmark infrastructure appropriate at implementation time.

Benchmark:

```text
lexer throughput
parser throughput
type checking
module resolution
incremental rebuild
code generation
package resolution
```

Performance claims should be measured.

---

# 49. Memory Profiling

When memory becomes significant, profile:

```text
AST memory
HIR memory
interned strings
type arena
source buffers
compiler caches
```

Do not guess which component consumes memory.

---

# 50. Code Generation Backend

Vut should use a proven code-generation backend rather than implement machine-code generation from scratch.

Primary candidate for fast native compilation:

```text
Cranelift
```

Evaluate according to Vut's final backend requirements.

Potential alternatives for specific future requirements may include LLVM-based approaches.

The backend must remain behind a Vut-owned abstraction.

Conceptually:

```text
Vut MIR
   ↓
CodegenBackend
   ├── CraneliftBackend
   └── future backend
```

Do not expose Cranelift-specific concepts throughout HIR/type checking.

---

# 51. Backend Abstraction

Codegen backend details belong inside:

```text
vut-codegen
```

Frontend modules must not directly manipulate backend instructions.

Bad:

```text
parser -> Cranelift IR
```

Correct:

```text
parser
  ↓
AST
  ↓
HIR
  ↓
typed representation
  ↓
MIR
  ↓
codegen backend
```

---

# 52. IR Must Have Clear Responsibilities

Do not use one giant AST for every compiler phase.

Recommended conceptual separation:

```text
AST
  syntax-oriented

HIR
  resolved semantic structure

MIR
  lower-level optimization/codegen representation
```

Each lowering step should remove unnecessary complexity from later phases.

---

# 53. AST Should Preserve Source Information

AST nodes should retain sufficient source spans for diagnostics.

Do not discard source location information during parsing.

Every important semantic node should be traceable back to source.

---

# 54. HIR Should Use Resolved IDs

HIR should avoid repeatedly storing unresolved names where semantic resolution has already happened.

Prefer:

```text
SymbolId
TypeId
ModuleId
```

over repeated string lookup.

---

# 55. Type System Must Be Centralized

Do not spread type compatibility logic throughout:

```text
parser
codegen
interfaces
runtime
```

Type rules belong in the type-system/type-checking layer.

Codegen consumes already checked types.

---

# 56. Interface Checking Must Be Centralized

Structural interface satisfaction should have one authoritative implementation.

Do not duplicate:

```text
method matching
signature compatibility
visibility checking
```

in multiple compiler modules.

---

# 57. Diagnostics Must Be First-Class

Do not treat diagnostics as:

```text
format!("error...")
```

strings scattered throughout compiler code.

Use structured diagnostics.

Conceptually:

```text
Diagnostic
├── code
├── severity
├── message
├── primary_span
├── labels
├── notes
└── help
```

Rendering happens separately.

---

# 58. Source Management Must Be Centralized

Source files should be managed through a dedicated source database/manager.

Conceptually:

```text
SourceId -> SourceFile
```

Store:

```text
path
text
line offsets
metadata
```

Avoid repeatedly reading files from disk in separate compiler stages.

---

# 59. Efficient Line Lookup

Diagnostics require:

```text
byte offset -> line/column
```

Do not scan from the beginning of the source for every diagnostic.

Precompute or lazily cache line-start offsets.

Use binary search or equivalent efficient lookup.

---

# 60. UTF-8 Must Be Handled Correctly

Vut source is UTF-8.

Never assume:

```text
byte index == character index
```

Spans may use byte offsets internally for efficiency, but line/column rendering must respect UTF-8 boundaries.

---

# 61. Avoid Panics for User Errors

Invalid Vut source must not crash the compiler.

Do not use:

```rust
unwrap()
expect()
panic!()
```

for conditions reachable from normal invalid user input.

These are acceptable only for genuine internal invariants where failure represents a compiler bug.

---

# 62. Internal Compiler Errors

Impossible internal states should become clear ICE diagnostics where practical.

They must be distinguishable from user source errors.

Never blame the user for an internal compiler invariant failure.

---

# 63. Parser Recovery Is Mandatory

The parser should recover from errors when safe.

Do not stop at the first missing colon or malformed expression if later declarations can still be parsed.

At the same time:

```text
do not generate cascading nonsense errors
```

Recovery must be intentional.

---

# 64. Tests Are Required With Implementation

Do not implement a major component and postpone all tests.

When implementing:

```text
lexer
parser
type checker
module resolver
interface checker
diagnostics
VPM resolver
```

add tests as part of the same work.

---

# 65. Every Bug Fix Should Add a Regression Test

When fixing a compiler bug:

```text
1. reproduce bug in test
2. confirm test fails
3. fix implementation
4. confirm test passes
```

Do not fix reproducible compiler bugs without regression coverage unless genuinely impractical.

---

# 66. Benchmark Performance-Critical Components

Before major architectural changes to hot compiler paths, create or update benchmarks.

Compare:

```text
before
after
```

Do not claim optimization without measurement.

---

# 67. Do Not Sacrifice Correctness for Benchmark Numbers

An optimization is invalid if it changes Vut semantics.

Priority:

```text
correctness
then performance
```

Performance must be pursued inside correct semantics.

---

# 68. Release Mode Matters

Performance-sensitive measurements should use appropriate optimized builds.

Do not judge runtime/compiler performance from Rust debug builds alone.

---

# 69. Avoid Premature Dynamic Dispatch

Use static dispatch where architecture naturally permits it.

Dynamic dispatch is acceptable where it provides a meaningful abstraction boundary.

Do not use:

```text
Box<dyn Trait>
```

everywhere merely for convenience.

Evaluate allocation and dispatch cost in hot paths.

---

# 70. Avoid Excessive `Arc<Mutex<...>>`

Do not solve ownership architecture by wrapping compiler state in:

```text
Arc<Mutex<T>>
```

without need.

Prefer clear ownership and immutable shared data.

Synchronization should exist only where actual concurrency requires it.

---

# 71. Prefer Immutable Compiler Phase Inputs

Where practical, compiler phases should consume immutable representations and produce new semantic outputs.

This makes:

```text
parallelism
incremental compilation
caching
testing
reasoning
```

easier.

Controlled mutation inside dedicated arenas/tables is acceptable.

---

# 72. Public APIs Must Be Small

Internal crates should expose only what other crates need.

Avoid:

```rust
pub
```

on everything.

Maintain strong encapsulation between compiler phases.

---

# 73. No Circular Crate Dependencies

Workspace architecture must form a clear dependency direction.

Conceptually:

```text
source
   ↓
lexer
   ↓
parser
   ↓
AST
   ↓
HIR/types
   ↓
middle
   ↓
codegen
```

Do not create cycles between compiler crates.

If two crates need each other, the architecture is probably wrong.

Extract the shared abstraction into a lower-level crate if appropriate.

---

# 74. Dependency Direction

High-level components may depend on lower-level abstractions.

Lower-level compiler components must not depend on CLI/UI layers.

Bad:

```text
lexer -> vut-cli
```

Correct:

```text
vut-cli -> vut-compiler -> lexer
```

---

# 75. VPM Must Be Separate From Compiler Core

VPM responsibilities:

```text
project management
package resolution
downloads
lockfile
manifest
cache
workflow
```

Compiler responsibilities:

```text
source compilation
semantic analysis
code generation
```

Do not put GitHub/GitLab HTTP code into compiler crates.

---

# 76. Provider Architecture

GitHub and GitLab support must use provider abstractions.

Conceptually:

```text
PackageProvider
├── GithubProvider
└── GitlabProvider
```

Do not duplicate package-resolution logic separately for each provider.

---

# 77. Filesystem Paths Must Use Native Path Types

Use:

```rust
Path
PathBuf
```

for filesystem paths.

Do not model filesystem paths as concatenated `/` strings internally.

Package identifiers and filesystem paths are different concepts.

---

# 78. Atomic File Writes

Important generated files such as:

```text
vpm.lock
compiler metadata
critical caches
```

should use atomic replacement where practical.

Conceptually:

```text
write temporary file
flush/validate
rename
```

Avoid leaving corrupted files after interruption.

---

# 79. Package Downloads Must Be Validated

Never trust downloaded package contents.

Validate:

```text
source
version
manifest
path safety
checksum/revision
archive extraction paths
```

Prevent path traversal such as:

```text
../../
```

during extraction.

---

# 80. Security Is Part of Correctness

VPM processes untrusted remote data.

Do not defer obvious security architecture until later.

Important areas:

```text
HTTP/TLS
archive extraction
filesystem writes
checksums
GitHub/GitLab API data
manifest parsing
package paths
```

---

# 81. Standard Library Should Prefer Rust/OS Capabilities Internally

Do not implement low-level functionality manually when Rust's standard library or proven crates already provide robust implementations.

Examples:

```text
filesystem
environment
networking
time
Unicode
path handling
```

Vut's public API may differ, but internal implementation should reuse proven foundations.

---

# 82. Platform Abstraction

Platform-specific code should be isolated.

Conceptually:

```text
platform/
├── windows.rs
├── linux.rs
└── macos.rs
```

or equivalent.

Do not scatter:

```rust
#[cfg(target_os = "...")]
```

through unrelated modules when a clean platform abstraction can contain it.

---

# 83. No Copy-Paste Platform Implementations

Extract shared logic.

Only genuinely platform-specific behavior belongs in platform modules.

---

# 84. Unsafe Rust Must Be Minimized

Compiler/runtime Rust implementation should prefer safe Rust.

Use `unsafe` only when:

```text
required for FFI
required for proven performance reasons
required for low-level runtime implementation
```

Every unsafe block must have a clear safety invariant.

---

# 85. Unsafe Requires Documentation

Each meaningful unsafe block should explain why it is safe.

Conceptually:

```rust
// SAFETY:
// ...
unsafe {
    ...
}
```

Do not leave unexplained unsafe operations in the runtime.

---

# 86. Do Not Use Unsafe as a Performance Guess

Do not introduce unsafe code merely because it might be faster.

Measure first unless unsafe is intrinsically required by the operation.

Safe Rust is the default.

---

# 87. Error Handling

Libraries should return structured errors.

CLI layers decide how errors are rendered.

Do not terminate the process from deep library code with:

```rust
std::process::exit(...)
```

Deep modules should return errors upward.

---

# 88. No Production Debug Prints

Before completing a phase, remove temporary:

```text
println!
dbg!
eprintln!
```

used for debugging unless they are part of intended CLI output.

Use tracing for internal diagnostics.

---

# 89. Warnings Must Be Clean

New implementation should not introduce avoidable compiler warnings.

Do not silence warnings globally to hide poor code.

Fix the cause.

---

# 90. Clippy

The Rust workspace should remain compatible with:

```text
cargo clippy
```

Treat important Clippy findings seriously.

Do not blindly accept every suggestion if it harms architecture, but do not ignore warnings without reason.

---

# 91. Formatting

Rust implementation must remain formatted with:

```text
cargo fmt
```

Do not commit intentionally unformatted Rust code.

---

# 92. Documentation

Important public/internal architectural APIs should have documentation explaining:

```text
responsibility
invariants
inputs
outputs
ownership expectations
```

Do not write comments that merely repeat obvious code.

Comments should explain:

```text
why
```

rather than:

```text
what this obvious line does
```

---

# 93. Naming

Names must describe responsibility.

Avoid vague modules such as:

```text
utils.rs
helpers.rs
common2.rs
misc.rs
stuff.rs
manager.rs
```

unless the contained concept is genuinely cohesive.

Prefer:

```text
source_map.rs
module_graph.rs
type_arena.rs
package_id.rs
version_resolver.rs
diagnostic_renderer.rs
```

---

# 94. Avoid Generic `utils`

Shared code should be grouped by domain.

Instead of:

```text
utils/
```

prefer:

```text
path/
hash/
source/
collections/
```

where appropriate.

---

# 95. No Duplicate Logic

If the same semantic logic appears in multiple places, extract one authoritative implementation.

Especially avoid duplication in:

```text
type compatibility
interface matching
module path normalization
SemVer handling
package identity
diagnostic rendering
source span calculation
```

---

# 96. Keep Compiler Phases Independent

Parser should not type-check.

Lexer should not resolve modules.

Type checker should not generate machine code.

Codegen should not reinterpret source syntax.

Each phase trusts validated output from the appropriate previous stages.

---

# 97. No Stringly-Typed Compiler Architecture

Avoid APIs such as:

```text
type = "int"
kind = "function"
visibility = "private"
```

Use enums/IDs:

```text
TypeKind
SymbolKind
Visibility
```

Strings are for user-facing names, not internal semantic state.

---

# 98. Enums for Finite States

Use Rust enums for finite compiler states.

Example:

```text
DiagnosticSeverity
SymbolKind
PackageSource
BuildMode
TokenKind
```

This allows exhaustive checking.

---

# 99. Newtypes for Important IDs

Prefer dedicated types:

```rust
struct SourceId(...);
struct ModuleId(...);
struct SymbolId(...);
struct TypeId(...);
```

rather than passing raw integers everywhere.

This prevents mixing unrelated IDs.

---

# 100. Stable Internal Invariants

Each compiler phase should define invariants.

Example:

After name resolution:

```text
all resolvable names have SymbolId
```

After type checking:

```text
all executable expressions have known checked types
```

After MIR lowering:

```text
high-level syntax constructs are removed
```

Later phases should rely on these invariants instead of rechecking everything.

---

# 101. Avoid Repeated Work Between Phases

Do not repeatedly calculate the same information.

If type checker resolves a type, codegen should consume its resolved `TypeId`.

Do not resolve the type name again from text.

---

# 102. Caching

Cache expensive reusable results when justified.

Potential examples:

```text
source hashes
parsed package metadata
line indexes
module resolution
dependency metadata
build artifacts
```

Caches must have explicit invalidation rules.

---

# 103. Cache Is Never Source of Truth

A corrupted or stale cache must never change program semantics.

The compiler/VPM should be able to delete caches and rebuild correct results.

---

# 104. Dependency Versions

Rust dependencies should normally be managed through Cargo workspace dependencies where shared across crates.

Avoid inconsistent versions of the same dependency across many workspace crates without reason.

---

# 105. Do Not Install Libraries Prematurely

Permission to use libraries does not mean:

```text
install everything at project start
```

Add dependencies when the corresponding feature is actually implemented.

This keeps compile time and dependency surface controlled.

---

# 106. Evaluate Dependency Cost

Before adding a large dependency for a tiny feature, consider whether a smaller proven crate or standard library already solves it.

The rule is not:

```text
always use library
```

The rule is:

```text
do not reimplement mature non-Vut-specific functionality without reason
```

---

# 107. Custom Implementation Is Allowed

A custom implementation is appropriate when:

```text
the behavior is Vut-specific
existing libraries do not fit
external dependency would impose unacceptable overhead
precise compiler control is required
benchmarks justify it
```

Examples may include:

```text
Vut type checker
Vut interface checker
Vut HIR
Vut MIR
Vut module semantics
Vut language-specific parser logic
```

These are core Vut intellectual/semantic components and should not be outsourced to generic libraries.

---

# 108. Benchmark Before Replacing Proven Libraries

Do not replace a mature dependency with custom code based only on assumptions that custom code will be faster.

Measure:

```text
speed
memory
compile time
binary size
complexity
maintenance cost
```

first.

---

# 109. No Temporary Bad Architecture

The following reasoning is prohibited:

```text
put everything in main.rs for now
we can split it later
```

```text
clone everything for now
we can optimize later
```

```text
use strings for all types for now
we can create TypeId later
```

```text
parse and type-check together for now
we can separate them later
```

```text
hard-code GitHub logic for now
we can add providers later
```

If the intended architecture is already known, implement the correct boundary immediately.

---

# 110. Incremental Delivery Does Not Mean Temporary Architecture

Vut should still be developed phase-by-phase.

A phase may implement only:

```text
integer lexer
```

or:

```text
basic expression parser
```

but that small feature must live inside the final-quality architecture.

Good:

```text
small feature
+
correct architecture
```

Bad:

```text
small feature
+
temporary architecture
+
future rewrite
```

---

# 111. Every Phase Must Leave the Repository Healthy

At the end of each implementation phase:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy
```

should pass where applicable.

Do not knowingly leave the repository broken for the next phase to repair.

---

# 112. Phase Completion Checklist

Before marking a roadmap phase complete, verify:

```text
[ ] implementation follows specs
[ ] modules are properly separated
[ ] no giant temporary files
[ ] no obvious duplicated logic
[ ] no unnecessary large clones
[ ] no unnecessary allocations in known hot paths
[ ] errors are structured
[ ] source spans are preserved
[ ] tests exist
[ ] regression tests exist where applicable
[ ] cargo fmt passes
[ ] cargo check passes
[ ] cargo test passes
[ ] cargo clippy is reviewed
[ ] no temporary debug code remains
[ ] no speculative language syntax was introduced
[ ] relevant specs/roadmap status updated
```

---

# 113. Performance Review Checklist

For performance-sensitive work, review:

```text
allocation count
clone count
algorithmic complexity
hash lookups
string allocations
filesystem operations
network operations
cache behavior
parallelization opportunities
data locality
```

Do not wait until the entire compiler exists before reviewing these fundamentals.

---

# 114. Parser Performance

Parser implementation should avoid:

```text
repeated backtracking
re-tokenizing input
repeated substring allocation
copying token streams
```

Expression parsing should use an efficient deterministic strategy such as Pratt parsing when appropriate.

---

# 115. Lexer Performance

Lexer should process source approximately linearly.

Avoid allocating a new string for every token when spans into the original source are sufficient.

Tokens should generally reference:

```text
SourceId
Span
TokenKind
```

rather than own duplicated source text.

---

# 116. Symbol Performance

Identifiers should eventually support efficient interning.

Symbol lookup should use appropriate indexed/hash structures.

Avoid repeated full-string scans across all scopes.

---

# 117. Type Checker Performance

Type checker should operate on compact internal type representations.

Avoid recursively reconstructing equivalent types repeatedly.

Intern/canonicalize types where beneficial.

Conceptually:

```text
TypeId -> TypeData
```

---

# 118. Module Resolver Performance

Build a module graph once per relevant compilation state.

Do not repeatedly scan the entire filesystem for every import.

Normalize and cache module identities.

---

# 119. VPM Performance

VPM should avoid:

```text
re-downloading unchanged packages
re-hashing unchanged content unnecessarily
re-querying providers unnecessarily
re-parsing unchanged manifests repeatedly
```

Use validated local caches.

---

# 120. Network Efficiency

VPM provider code should:

```text
reuse connections where library supports it
avoid redundant API calls
cache metadata appropriately
respect provider rate limits
```

Do not issue one remote request per trivial local operation when information can be fetched efficiently.

---

# 121. Build Performance

Compiler architecture should eventually allow:

```text
incremental compilation
parallel independent work
cached dependency builds
source hashing
dependency graph invalidation
```

Do not design data structures that inherently prevent these features.

---

# 122. Runtime Performance

Runtime operations used by common Vut programs must avoid unnecessary overhead.

Particular attention should eventually be paid to:

```text
str
list
data
interface dispatch
dyn
allocation
```

Do not make all Vut values boxed merely because implementation is easier.

---

# 123. Zero-Cost Principle

Features that are not used should impose minimal runtime cost.

Examples:

```text
no dyn -> no dynamic type checks
no interfaces -> no interface dispatch
no concurrency -> no scheduler
no async -> no executor
no FFI -> no FFI runtime layer
```

Avoid global runtime overhead for unused features.

---

# 124. Static Information Should Be Used

Vut is statically typed.

The compiler should exploit known static information for:

```text
direct calls
layout
specialization where appropriate
dead code elimination opportunities
type-safe lowering
```

Do not unnecessarily defer known information to runtime.

---

# 125. `dyn` Must Pay Its Own Cost

Dynamic behavior belongs to:

```text
dyn
```

Statically typed code should not inherit unnecessary dynamic-dispatch/type-tagging overhead because `dyn` exists.

---

# 126. Interface Cost Must Be Localized

Concrete method calls should remain direct when possible.

Interface dispatch overhead should occur when values are actually used through interface abstraction.

Do not route every method call through a universal runtime dispatcher.

---

# 127. Avoid Universal Object Representation

Do not represent every Vut value as something equivalent to:

```text
Object
Value
Box<dyn Any>
```

unless required by a specific dynamic boundary.

Primitive and statically known values should retain efficient native representations.

---

# 128. Release Optimization

The compiler should eventually support optimized release builds.

Optimization pipeline belongs in dedicated compiler stages.

Do not mix optimization transformations into parser/type-checker logic.

---

# 129. Measure Against Real Programs

As Vut matures, benchmarks should include:

```text
small CLI program
numeric workload
collection workload
string workload
large multi-module project
interface-heavy workload
dyn-heavy workload
```

Microbenchmarks alone are insufficient.

---

# 130. Compare Performance Carefully

When comparing Vut against languages such as:

```text
C
Rust
Go
```

use equivalent workloads and optimized builds.

Do not make performance claims based on incomparable code or debug builds.

---

# 131. Dependency Security

Before introducing important dependencies, check:

```text
maintenance
known security advisories
license
release activity
dependency tree
```

Security-sensitive VPM dependencies deserve extra review.

---

# 132. Dependency Updates

Do not automatically update every dependency to a new major version without review.

For significant dependency upgrades:

```text
review changelog
run tests
run benchmarks when performance-sensitive
check API changes
```

---

# 133. Avoid Unnecessary Feature Flags

Disable unnecessary crate features when they pull large dependency trees or functionality Vut does not use.

Prefer intentional Cargo feature selection.

---

# 134. Platform Portability

Core compiler logic should remain platform-independent.

Platform-specific behavior belongs behind dedicated abstractions.

Vut must not accidentally become Windows-only or Unix-only because path/process assumptions leaked into core logic.

---

# 135. No Shell Command Dependence for Core Logic

Do not rely on shell utilities such as:

```text
grep
sed
awk
curl
tar
rm
cp
```

for core cross-platform compiler/VPM behavior when Rust libraries provide portable equivalents.

External toolchain commands such as system linker invocation are acceptable where part of the compilation model.

---

# 136. Windows Paths Must Work

Code must correctly handle:

```text
C:\...
```

and other Windows path behavior.

Do not split filesystem paths manually on `/`.

---

# 137. Unix Paths Must Work

Likewise, Unix paths and permissions must be handled appropriately.

Platform-specific behavior should be tested.

---

# 138. No Hidden Environment Dependence

Compiler results should not depend on random environment variables unless explicitly part of the build configuration.

Environment-dependent inputs should be made explicit where practical.

---

# 139. API Boundaries Must Be Testable

Important subsystems should be testable without launching the full CLI.

Example:

Good:

```text
Resolver::resolve(...)
TypeChecker::check(...)
Parser::parse(...)
```

Bad:

```text
tests must execute vut.exe for every internal operation
```

CLI integration tests are still required separately.

---

# 140. CLI Is a Thin Layer

`vut` and `vpm` CLI crates should mainly:

```text
parse arguments
construct configuration
invoke library APIs
render result
choose exit code
```

Business/compiler logic belongs in reusable libraries.

---

# 141. No Business Logic in `main.rs`

`main.rs` should remain small.

It must not become the implementation of:

```text
compiler
package manager
resolver
formatter
```

---

# 142. Keep Public Output Intentional

CLI output is part of user experience.

Do not print internal debug representations such as:

```text
Debug AST
Rust error chain
raw provider response
```

unless explicit debug/verbose modes request them.

---

# 143. Diagnostics Must Remain Beautiful

All source-related errors must follow:

```text
specs/09-errors-diagnostics.md
specs/22-error-codes.md
```

Do not regress to:

```text
Error: bad type
```

when precise source context exists.

---

# 144. No Fake Diagnostics

If an error is not source-related, do not fabricate:

```text
file
line
column
```

Example:

```text
package version not found
```

should use VPM's structured package error layout.

---

# 145. Keep Source Spans Through Lowering

When AST becomes HIR/MIR, retain source origins where useful.

Codegen/runtime errors and later semantic diagnostics should still be traceable to meaningful source locations.

---

# 146. Architecture Changes Require Review

If implementation discovers that an existing specification implies a poor architecture, do not silently redesign everything.

Instead:

```text
identify issue
document proposed change
update relevant spec after approval
then implement
```

---

# 147. Do Not Modify Unrelated Features

While implementing one phase, avoid opportunistically rewriting unrelated working modules unless necessary.

Keep changes focused and reviewable.

---

# 148. Refactor When Responsibility Changes

Refactoring is appropriate when new implementation reveals that an existing module has accumulated multiple responsibilities.

Do not postpone obvious architectural cleanup indefinitely.

But refactoring must preserve tests and semantics.

---

# 149. Keep Repository Structure Clean

Do not leave:

```text
old/
backup/
tmp/
test2/
new_parser/
parser_old/
final2/
```

directories.

Git provides history.

Remove superseded implementation after replacement is verified.

---

# 150. No Dead Experimental Code

Do not leave large commented-out implementations.

Use version control instead.

Temporary experiments should be removed before phase completion.

---

# 151. TODO Policy

TODO comments are allowed only for genuine deferred work.

Good:

```text
TODO: incremental invalidation after phase 12
```

Bad:

```text
TODO: fix architecture later
TODO: optimize this entire subsystem later
TODO: add error handling
```

Core correctness cannot be deferred behind TODO.

---

# 152. Performance TODO Policy

A known non-optimal implementation may remain temporarily only when:

```text
it does not compromise architecture
the optimization requires later unavailable information
the limitation is documented
a benchmark/test exists where appropriate
```

Do not use this exception to justify obviously poor foundational design.

---

# 153. Existing Libraries Should Be Preferred Intelligently

Before implementing generic infrastructure, Codex should ask internally:

```text
Does Rust std already provide this?

Does a mature crate provide this?

Would custom implementation actually provide Vut-specific value?

What are the performance implications?

What are the maintenance implications?
```

If a proven crate is clearly better, install and use it automatically.

No additional user permission is required for normal implementation dependencies.

---

# 154. Codex May Modify Project Structure

Codex is explicitly allowed to:

```text
create files
create folders
create crates
split modules
move implementation
add dependencies
add tests
add benchmarks
add internal tooling
```

when required to maintain clean architecture.

Do not keep bad structure merely because the initial repository started small.

---

# 155. Codex Must Preserve Public Design

Codex may freely improve internal architecture.

Codex may NOT independently change:

```text
Vut syntax
Vut semantics
package format
CLI contract
visibility rules
type rules
module rules
error behavior
```

that have already been specified.

Internal freedom does not mean public-language freedom.

---

# 156. Preferred Initial Technical Stack

The following is the preferred starting stack, subject to measured architectural review:

```text
Language implementation:
  Rust

CLI:
  clap

Serialization:
  serde

TOML:
  toml
  toml_edit

SemVer:
  semver

Lexer:
  logos or optimized custom lexer

Parser:
  structured recursive descent + Pratt
  OR a proven parser crate after evaluation

Lossless syntax tree if needed:
  rowan

Diagnostics:
  codespan-reporting / ariadne / annotate-snippets
  behind Vut-owned diagnostic abstraction

Errors:
  thiserror
  anyhow only at suitable application boundaries

HTTP:
  reqwest

URL:
  url

Hashing:
  blake3
  sha2 when interoperability requires SHA-2

Filesystem traversal:
  ignore
  walkdir

Temporary files:
  tempfile

Parallel CPU work:
  rayon

Graph algorithms:
  petgraph where appropriate

Ordered maps:
  indexmap

Fast compiler-internal maps:
  rustc-hash or hashbrown when justified

Arenas:
  la-arena / bumpalo / equivalent after architecture review

String interning:
  lasso / string-interner after measurement/design review

Tracing:
  tracing
  tracing-subscriber

Benchmarking:
  criterion or appropriate stable benchmark tooling

Native code generation:
  Cranelift as primary candidate

Async networking:
  tokio only if VPM architecture actually requires async I/O
```

This is not a requirement to install all of them.

---

# 157. Libraries That Must Not Be Reimplemented Casually

Codex should require a strong technical reason before replacing mature implementations of:

```text
SemVer
TOML
HTTP
TLS
URL parsing
cryptographic hashing
CLI parsing
archive handling
Unicode processing
parallel scheduling
temporary file handling
terminal styling
```

These are not core Vut language innovations.

---

# 158. Components Vut Should Own

Vut should own its language-specific core:

```text
Vut grammar behavior
AST design
HIR design
MIR design
type inference
type checking
structural interface checking
Vut module semantics
Vut visibility semantics
Vut control-flow semantics
Vut diagnostics model
Vut package identity rules
Vut runtime representation
Vut optimization strategy
```

Libraries may support these components but must not define Vut's semantics.

---

# 159. Final Quality Rule

A feature is NOT complete merely because:

```text
it compiles
```

or:

```text
the example works
```

A feature is complete when:

```text
behavior is correct
architecture is clean
module boundaries are appropriate
tests exist
errors are good
performance implications were considered
unnecessary allocations/copies were avoided
existing libraries were reused where appropriate
specification is respected
repository remains healthy
```

---

# 160. Fundamental Rule

The most important engineering rule for Vut is:

> Do not build temporary bad architecture with the intention of fixing it later.

Instead:

> Implement each phase as a small but production-quality part of the final architecture.

Development should progress like:

```text
small
→ correct
→ tested
→ modular
→ efficient
→ complete
```

not:

```text
quick hack
→ more hacks
→ large file
→ technical debt
→ rewrite later
```

Every new piece of Vut should leave the project in a better, maintainable and performance-conscious state.

The target is not merely:

```text
Vut works.
```

The target is:

```text
Vut is correct.
Vut is fast.
Vut is maintainable.
Vut is scalable.
Vut is built properly from the beginning.
```
