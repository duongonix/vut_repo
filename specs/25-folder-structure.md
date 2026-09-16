# Vut Folder Structure

## 1. Purpose

This document defines the recommended repository and Rust workspace structure for Vut.

The goal is to keep the compiler:

* modular
* maintainable
* scalable
* easy to test
* easy to optimize
* easy to refactor
* resistant to giant files
* suitable for long-term compiler development

This structure is a strong architectural guideline.

Codex may create additional files, folders, modules or crates when needed, as long as the resulting architecture remains clean and follows the rules in this document.

---

# 2. Core Rule

Do not place too much unrelated logic into one file.

Avoid files that continuously grow into:

```text
2000 lines
4000 lines
8000 lines
```

just because adding another module was inconvenient.

When a file starts handling multiple responsibilities, split it.

Prefer:

```text
one clear responsibility
per file/module
```

over:

```text
one giant file
containing an entire compiler subsystem
```

---

# 3. Codex Is Allowed to Create Structure

Codex is explicitly allowed to create:

```text
new crates
new folders
new Rust modules
new helper files
new test directories
new benchmark directories
new internal abstractions
```

without asking for permission first when they improve architecture.

Codex should proactively split code when necessary.

Do not wait until a file becomes unmaintainable.

---

# 4. Structure Is Not Frozen

The folder structure in this document is the preferred architecture, not a restriction against future improvements.

Codex may add:

```text
new files
new submodules
new internal directories
new crates
```

when required by implementation complexity.

However:

* do not reorganize architecture arbitrarily
* do not create unnecessary abstraction layers
* do not create crates for tiny helpers
* do not move responsibilities across layers without good reason
* keep dependency direction clean

---

# 5. Repository Structure

Recommended top-level layout:

```text
vut/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
├── LICENSE
│
├── crates/
│   ├── vut-source/
│   ├── vut-diagnostics/
│   ├── vut-lexer/
│   ├── vut-ast/
│   ├── vut-parser/
│   ├── vut-symbols/
│   ├── vut-resolver/
│   ├── vut-hir/
│   ├── vut-types/
│   ├── vut-interface/
│   ├── vut-memory/
│   ├── vut-mir/
│   ├── vut-optimizer/
│   ├── vut-codegen/
│   ├── vut-linker/
│   ├── vut-runtime/
│   ├── vut-std/
│   ├── vut-incremental/
│   ├── vut-compiler/
│   ├── vut-cli/
│   └── vpm/
│
├── tests/
│   ├── compile-pass/
│   ├── compile-fail/
│   ├── diagnostics/
│   ├── runtime/
│   ├── packages/
│   └── integration/
│
├── benches/
│   ├── lexer/
│   ├── parser/
│   ├── typecheck/
│   ├── runtime/
│   └── compiler/
│
├── specs/
│   └── ...
│
└── examples/
    └── ...
```

Not every crate must exist immediately.

Create crates when their roadmap phase begins or when architecture genuinely requires them.

Do not create empty crates merely to match this tree.

---

# 6. Architectural Layers

The compiler should broadly follow these layers:

```text
Frontend
├── vut-source
├── vut-diagnostics
├── vut-lexer
├── vut-ast
├── vut-parser
└── vut-resolver

Semantic
├── vut-symbols
├── vut-hir
├── vut-types
└── vut-interface

Middle-End
├── vut-memory
├── vut-mir
└── vut-optimizer

Backend
├── vut-codegen
├── vut-linker
└── vut-runtime

Tooling
├── vut-incremental
├── vut-compiler
├── vut-cli
└── vpm
```

These layers should remain conceptually separated.

---

# 7. Dependency Direction

Dependencies should primarily flow in one direction.

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
resolver
↓
HIR
↓
types
↓
interface checking
↓
memory analysis
↓
MIR
↓
optimizer
↓
codegen
↓
linker
```

`vut-compiler` orchestrates the pipeline.

`vut-cli` calls `vut-compiler`.

`vpm` calls compiler APIs rather than duplicating compiler logic.

Avoid circular crate dependencies.

---

# 8. Shared Types

If multiple compiler layers require the same foundational semantic IDs or interned symbols, place them in a lower-level shared crate instead of creating circular dependencies.

Example:

```text
vut-symbols
```

may contain:

```text
Symbol
SymbolId
StringInterner
common semantic identifiers
```

where appropriate.

Do not create dependency cycles such as:

```text
vut-hir
→ vut-types
→ vut-hir
```

If this happens, move shared representations into an appropriate lower-level crate.

---

# 9. `vut-source`

Recommended structure:

```text
crates/vut-source/
└── src/
    ├── lib.rs
    ├── source_file.rs
    ├── source_map.rs
    ├── source_id.rs
    ├── span.rs
    └── line_index.rs
```

Responsibilities:

```text
source loading
source identity
Span
file metadata
line indexing
source slicing
line/column mapping
```

Do not include lexer or parser logic here.

---

# 10. `vut-diagnostics`

Recommended:

```text
crates/vut-diagnostics/
└── src/
    ├── lib.rs
    ├── diagnostic.rs
    ├── code.rs
    ├── label.rs
    ├── sink.rs
    └── renderer/
        ├── mod.rs
        ├── terminal.rs
        ├── plain.rs
        └── json.rs
```

Responsibilities:

```text
structured diagnostics
diagnostic codes
labels
notes
help
source-frame rendering
terminal rendering
plain rendering
machine-readable rendering
```

Compiler stages should construct structured diagnostics.

They must not directly emit ANSI-formatted strings.

---

# 11. `vut-lexer`

Recommended:

```text
crates/vut-lexer/
└── src/
    ├── lib.rs
    ├── lexer.rs
    ├── token.rs
    ├── token_kind.rs
    ├── number.rs
    ├── string.rs
    ├── template.rs
    ├── comment.rs
    └── indentation.rs
```

Split responsibilities when lexer complexity grows.

For example:

```text
string.rs
```

should not contain all number/comment/indentation logic.

Template string handling should preferably have its own module when it becomes non-trivial.

---

# 12. `vut-ast`

Recommended:

```text
crates/vut-ast/
└── src/
    ├── lib.rs
    ├── file.rs
    ├── item.rs
    ├── declaration.rs
    ├── statement.rs
    ├── expression.rs
    ├── types.rs
    └── template.rs
```

Responsibilities:

```text
syntax-oriented AST structures
source spans
syntax nodes
template-string syntax representation
```

Do not put semantic resolution/type checking here.

---

# 13. `vut-parser`

Recommended:

```text
crates/vut-parser/
└── src/
    ├── lib.rs
    ├── parser.rs
    ├── declaration.rs
    ├── function.rs
    ├── method.rs
    ├── statement.rs
    ├── expression.rs
    ├── types.rs
    ├── template.rs
    └── recovery.rs
```

Do not put the entire parser into:

```text
parser.rs
```

if it becomes large.

For example:

```text
declaration.rs
```

may handle declaration dispatch.

```text
function.rs
```

may parse:

```vut
fn add():
```

```text
method.rs
```

may parse:

```vut
fn User.add():
```

```text
expression.rs
```

should own Pratt expression parsing.

```text
recovery.rs
```

should centralize parser recovery.

---

# 14. `vut-symbols`

Recommended:

```text
crates/vut-symbols/
└── src/
    ├── lib.rs
    ├── symbol.rs
    ├── interner.rs
    └── ids.rs
```

Use this crate only for foundational shared symbol infrastructure.

Do not turn it into a dumping ground for unrelated compiler structures.

---

# 15. `vut-resolver`

Recommended:

```text
crates/vut-resolver/
└── src/
    ├── lib.rs
    ├── resolver.rs
    ├── module.rs
    ├── module_graph.rs
    ├── import.rs
    ├── scope.rs
    ├── symbol_table.rs
    ├── method.rs
    └── privacy.rs
```

Responsibilities include:

```text
module discovery
imports
relative imports
scopes
name lookup
privacy
module graph
cycle detection
method receiver resolution
implicit self scope
```

Do not allow `resolver.rs` to absorb all implementation.

---

# 16. `vut-hir`

Recommended:

```text
crates/vut-hir/
└── src/
    ├── lib.rs
    ├── hir.rs
    ├── item.rs
    ├── expression.rs
    ├── statement.rs
    ├── function.rs
    ├── method.rs
    ├── data.rs
    ├── interface.rs
    ├── ids.rs
    └── lowering.rs
```

When lowering becomes large, split further:

```text
lowering/
├── mod.rs
├── declarations.rs
├── expressions.rs
├── statements.rs
└── methods.rs
```

instead of leaving thousands of lines inside one `lowering.rs`.

---

# 17. `vut-types`

Recommended:

```text
crates/vut-types/
└── src/
    ├── lib.rs
    ├── type_id.rs
    ├── type_data.rs
    ├── interner.rs
    ├── inference.rs
    ├── compatibility.rs
    ├── layout.rs
    └── checker/
        ├── mod.rs
        ├── expression.rs
        ├── statement.rs
        ├── function.rs
        ├── method.rs
        ├── data.rs
        └── control_flow.rs
```

Do not build the entire type checker in:

```text
type_checker.rs
```

Type checking will become one of the largest compiler subsystems.

Split by semantic domain early.

---

# 18. `vut-interface`

Recommended:

```text
crates/vut-interface/
└── src/
    ├── lib.rs
    ├── checker.rs
    ├── shape.rs
    ├── composition.rs
    ├── satisfaction.rs
    └── cache.rs
```

Responsibilities:

```text
structural compatibility
interface shape normalization
interface composition
satisfaction checking
satisfaction caching
```

Do not mix runtime vtable code here.

Runtime interface representation belongs later in runtime/codegen.

---

# 19. `vut-memory`

Recommended:

```text
crates/vut-memory/
└── src/
    ├── lib.rs
    ├── ownership.rs
    ├── last_use.rs
    ├── move_analysis.rs
    ├── drop_analysis.rs
    ├── cleanup.rs
    └── escape.rs
```

Responsibilities:

```text
ownership reasoning
last-use analysis
move decisions
drop requirements
cleanup planning
escape information
```

Follow:

```text
specs/08-memory-model.md
```

Do not place all ownership logic into the type checker or MIR builder.

`escape.rs` may initially contain only required analysis infrastructure.

Full optimization belongs to later phases.

---

# 20. `vut-mir`

Recommended:

```text
crates/vut-mir/
└── src/
    ├── lib.rs
    ├── mir.rs
    ├── function.rs
    ├── block.rs
    ├── instruction.rs
    ├── value.rs
    ├── place.rs
    ├── builder.rs
    └── lowering/
        ├── mod.rs
        ├── expression.rs
        ├── control_flow.rs
        ├── calls.rs
        └── cleanup.rs
```

MIR should explicitly support ownership-related operations such as:

```text
Move
Copy
Drop
Allocate
```

Do not combine all MIR definitions, construction and lowering logic into one file.

---

# 21. `vut-optimizer`

Recommended:

```text
crates/vut-optimizer/
└── src/
    ├── lib.rs
    ├── pipeline.rs
    ├── constant_fold.rs
    ├── constant_propagation.rs
    ├── dce.rs
    ├── copy_elision.rs
    ├── drop_elimination.rs
    ├── escape_analysis.rs
    ├── stack_promotion.rs
    ├── scalar_replacement.rs
    ├── bounds.rs
    └── devirtualization.rs
```

Do not create a separate crate for every optimization pass.

For example, do not create:

```text
vut-dce
vut-copy-elision
vut-stack-promotion
```

unless future architecture provides a very strong reason.

Optimization passes belong together under:

```text
vut-optimizer
```

---

# 22. `vut-codegen`

Recommended:

```text
crates/vut-codegen/
└── src/
    ├── lib.rs
    ├── backend.rs
    ├── context.rs
    ├── abi.rs
    ├── layout.rs
    ├── symbol.rs
    ├── target.rs
    └── cranelift/
        ├── mod.rs
        ├── backend.rs
        ├── function.rs
        ├── instruction.rs
        ├── calls.rs
        ├── data.rs
        └── runtime.rs
```

Responsibilities:

```text
MIR → backend IR
target ABI
object emission
runtime call lowering
native symbol generation
```

Do not generate machine code directly from AST/HIR.

---

# 23. Backend Abstraction

Backend-specific code must stay under its backend module.

Example:

```text
cranelift/
```

Avoid spreading Cranelift-specific types throughout:

```text
HIR
type checker
resolver
MIR
```

This preserves the ability to add another backend later.

---

# 24. `vut-linker`

Recommended:

```text
crates/vut-linker/
└── src/
    ├── lib.rs
    ├── linker.rs
    ├── platform.rs
    └── command.rs
```

Responsibilities:

```text
object files
runtime libraries
platform linker invocation
native executable output
```

Keep this separate from code generation.

---

# 25. `vut-runtime`

Recommended:

```text
crates/vut-runtime/
└── src/
    ├── lib.rs
    ├── abi.rs
    ├── allocator.rs
    ├── string.rs
    ├── bytes.rs
    ├── list.rs
    ├── map.rs
    ├── dyn_value.rs
    ├── interface.rs
    ├── panic.rs
    ├── bounds.rs
    └── io.rs
```

Runtime must remain small.

Do not move compiler semantics into the runtime simply because implementation is easier.

Prefer compile-time resolution whenever possible.

---

# 26. `vut-std`

Recommended:

```text
crates/vut-std/
└── src/
    ├── lib.rs
    ├── core/
    ├── collections/
    ├── string/
    ├── io/
    ├── fs/
    ├── env/
    ├── math/
    ├── time/
    └── result_option/
```

Each standard-library domain may contain additional files.

For example:

```text
fs/
├── mod.rs
├── file.rs
├── path.rs
├── metadata.rs
└── error.rs
```

if complexity requires it.

Do not force an entire std module into one file.

---

# 27. `vut-incremental`

Recommended:

```text
crates/vut-incremental/
└── src/
    ├── lib.rs
    ├── fingerprint.rs
    ├── cache.rs
    ├── cache_key.rs
    ├── dependency_graph.rs
    ├── invalidation.rs
    └── storage.rs
```

Responsibilities:

```text
fingerprints
cache keys
artifact cache
dependency invalidation
cache storage
incremental metadata
```

Compiler pipeline decides what to cache.

This crate provides infrastructure.

---

# 28. `vut-compiler`

Recommended:

```text
crates/vut-compiler/
└── src/
    ├── lib.rs
    ├── compiler.rs
    ├── session.rs
    ├── config.rs
    ├── pipeline.rs
    ├── build.rs
    ├── check.rs
    └── artifact.rs
```

This crate is the orchestration layer.

It connects:

```text
source
→ lexer
→ parser
→ resolver
→ HIR
→ type checker
→ ownership/MIR
→ optimizer
→ codegen
→ linker
```

Do not implement individual compiler subsystems here.

For example, `pipeline.rs` may call:

```text
lexer crate
parser crate
resolver crate
```

but must not contain their implementation.

---

# 29. `vut-cli`

Recommended:

```text
crates/vut-cli/
└── src/
    ├── main.rs
    ├── cli.rs
    ├── output.rs
    └── command/
        ├── mod.rs
        ├── run.rs
        └── build.rs
```

`main.rs` should be very small.

Conceptually:

```text
parse CLI
↓
dispatch command
↓
call compiler API
↓
render result
```

Do not put compiler logic inside `main.rs`.

---

# 30. `vpm`

Recommended:

```text
crates/vpm/
└── src/
    ├── main.rs
    ├── cli.rs
    ├── error.rs
    │
    ├── project/
    │   ├── mod.rs
    │   ├── manifest.rs
    │   └── lockfile.rs
    │
    ├── dependency/
    │   ├── mod.rs
    │   ├── resolver.rs
    │   ├── graph.rs
    │   └── version.rs
    │
    ├── provider/
    │   ├── mod.rs
    │   ├── github.rs
    │   ├── gitlab.rs
    │   └── registry.rs
    │
    ├── store/
    │   ├── mod.rs
    │   ├── packages.rs
    │   ├── cache.rs
    │   └── paths.rs
    │
    └── command/
        ├── mod.rs
        ├── new.rs
        ├── init.rs
        ├── add.rs
        ├── remove.rs
        ├── install.rs
        ├── update.rs
        ├── run.rs
        ├── build.rs
        └── check.rs
```

As new commands are implemented, create separate command modules.

Do not let:

```text
cli.rs
```

or:

```text
main.rs
```

contain all package-manager behavior.

---

# 31. Tests

Top-level integration tests should be separated by purpose.

Recommended:

```text
tests/
├── compile-pass/
├── compile-fail/
├── diagnostics/
├── runtime/
├── packages/
└── integration/
```

Subsystem unit tests may stay close to the implementation.

Use top-level test directories for end-to-end behavior.

---

# 32. Compile-Pass Tests

Examples:

```text
tests/compile-pass/
├── functions/
├── methods/
├── data/
├── interfaces/
├── loops/
├── imports/
└── templates/
```

Add more directories as language grows.

---

# 33. Compile-Fail Tests

Recommended:

```text
tests/compile-fail/
├── syntax/
├── types/
├── methods/
├── imports/
├── interfaces/
└── memory/
```

Example method regression:

```vut
User.greet():
  ...
```

must remain a compile-fail test because methods require `fn`.

---

# 34. Diagnostics Tests

Diagnostic snapshot tests should live separately where useful:

```text
tests/diagnostics/
```

Test:

```text
error code
message
span
source frame
expected/found
help
```

Avoid tests that only assert a generic `"error"` substring.

---

# 35. Benchmarks

Recommended:

```text
benches/
├── lexer/
├── parser/
├── typecheck/
├── runtime/
└── compiler/
```

Add benchmark groups as performance-sensitive subsystems mature.

Do not mix benchmarks into normal unit-test modules when dedicated benchmark harnesses are more appropriate.

---

# 36. Crate Creation by Phase

Do not create every future crate during Phase 01.

Recommended progression:

```text
Phase 01
├── vut-source
├── vut-diagnostics
└── vut-compiler foundation

Phase 02
└── vut-lexer

Phase 03
├── vut-ast
└── vut-parser

Phase 05
├── vut-symbols if required
└── vut-resolver

Phase 06
└── vut-hir

Phase 07
└── vut-types

Phase 11
└── vut-interface

Phase 12
├── vut-memory
└── vut-mir

Phase 13
├── vut-codegen
└── vut-linker

Phase 14
└── vut-runtime

Phase 15
└── vut-std

Phase 19
├── vut-optimizer
└── vut-incremental
```

This progression is guidance.

If a crate is architecturally needed earlier, Codex may create it earlier.

---

# 37. Avoid Empty Architecture

Do not create:

```text
20 empty crates
50 placeholder modules
unused traits
fake abstraction layers
```

solely because this document lists them.

Create a component when:

```text
it has a real responsibility
or
current architecture clearly requires it
```

---

# 38. File Size Rule

There is no rigid maximum number of lines per file.

However, Codex must actively monitor file responsibility and complexity.

Strong warning signs include:

```text
one file handles many unrelated concepts
one file has many independent sections
one file contains several major algorithms
one file changes for unrelated features
one file becomes difficult to navigate
tests for many unrelated behaviors sit together
```

When this happens, split the file.

Do not wait for an arbitrary line count.

---

# 39. Practical Size Guideline

As a practical guideline, review a file for splitting when it grows beyond roughly:

```text
500–800 lines
```

especially if it contains multiple responsibilities.

A file exceeding this range is not automatically wrong.

A tightly focused generated table or simple data declaration may legitimately be larger.

Conversely, a 250-line file may already need splitting if it contains several unrelated responsibilities.

Responsibility matters more than raw line count.

---

# 40. No Giant `mod.rs`

Do not move thousands of lines into:

```text
mod.rs
```

to claim the subsystem is modular.

`mod.rs` should primarily:

```text
declare modules
re-export public API
contain small shared glue
```

Major implementations belong in dedicated files.

---

# 41. No Giant `lib.rs`

Likewise:

```text
lib.rs
```

should expose the crate API and wire modules together.

Do not place the entire implementation of a crate in `lib.rs`.

---

# 42. No Giant `main.rs`

For executables such as:

```text
vut-cli
vpm
```

`main.rs` should remain minimal.

Prefer:

```rust
fn main() {
    ...
}
```

as startup/dispatch logic only.

Command behavior belongs in dedicated modules.

---

# 43. Split by Responsibility

Good:

```text
parser/
├── declaration.rs
├── expression.rs
├── statement.rs
└── recovery.rs
```

Bad:

```text
parser.rs
```

containing:

```text
token handling
declarations
expressions
statements
error recovery
AST construction
name resolution
```

---

# 44. Avoid Excessive Fragmentation

Do not split trivial logic into dozens of tiny one-function files.

Bad:

```text
parser/
├── parse_if.rs
├── parse_for.rs
├── parse_break.rs
├── parse_continue.rs
├── parse_return.rs
├── parse_add.rs
├── parse_sub.rs
...
```

unless complexity actually justifies it.

Prefer meaningful subsystem boundaries.

---

# 45. Internal Subfolders

When one subsystem grows, Codex should create subfolders.

Example:

```text
checker/
├── mod.rs
├── expression.rs
├── function.rs
├── method.rs
├── data.rs
└── control_flow.rs
```

rather than continuing to expand:

```text
checker.rs
```

---

# 46. Public API Boundaries

Each crate should expose the smallest reasonable public API.

Prefer:

```text
pub(crate)
private
```

internals where possible.

Do not make structures public merely to bypass architecture problems.

A clean dependency API is preferable to exposing internal implementation details.

---

# 47. Circular Dependency Rule

Circular crate dependencies are forbidden.

If two crates need each other's internal types, reconsider the abstraction.

Typical solutions:

```text
move shared IDs to lower-level crate
introduce a small shared abstraction crate
change ownership of a type
use interfaces/traits at an appropriate boundary
```

Do not use duplicated structures as a quick workaround.

---

# 48. Performance Rule

Folder structure must not force unnecessary runtime abstractions.

Architectural modularity does not mean:

```text
heap allocate every compiler object
Box everything
dynamic dispatch everywhere
Arc<Mutex<...>> everywhere
```

Prefer:

```text
static dispatch
compact IDs
arenas
borrowing inside Rust implementation
efficient data structures
batch processing
```

where appropriate.

---

# 49. Compiler Data Boundaries

Compiler phases should exchange explicit data models.

Example:

```text
lexer
→ tokens

parser
→ AST

resolver
→ resolved semantic information

HIR lowering
→ HIR

type checker
→ typed semantic tables

MIR lowering
→ MIR
```

Avoid hidden cross-layer mutations where possible.

---

# 50. No Architecture Shortcuts

Do not skip a planned layer merely because directly calling another layer is temporarily easier.

Examples of forbidden shortcuts:

```text
AST → Cranelift directly
parser performing type checking
type checker performing linker work
runtime parsing Vut expressions
VPM duplicating compiler pipeline
```

---

# 51. New Files Without Permission

Codex does not need explicit permission to create a new file when:

```text
existing file is becoming too large
logic has a clearly separate responsibility
a new subsystem is introduced
tests need their own module
platform-specific code should be separated
backend-specific code should be isolated
```

Create the file and integrate it cleanly.

---

# 52. New Folders Without Permission

Codex may introduce a new subfolder when a group of related modules forms a clear subsystem.

Example:

```text
codegen/
└── cranelift/
```

or:

```text
types/
└── checker/
```

Do not ask the user before performing routine architectural organization.

---

# 53. New Crates

Creating a new crate requires a stronger architectural reason than creating a file.

Good reasons include:

```text
clear independent compiler layer
dependency boundary
separate executable
runtime isolation
reusable infrastructure
compile-time dependency control
```

Do not create a new crate simply because one source file became large.

First consider splitting into modules inside the existing crate.

---

# 54. Refactoring Rule

When implementing a new feature reveals that an existing file or module has the wrong responsibility:

1. refactor the existing code
2. move code to the appropriate module
3. preserve behavior
4. update imports
5. update tests
6. continue feature implementation

Do not keep piling new code onto a poor architecture to avoid refactoring.

---

# 55. Tests During Refactoring

Structural refactoring must not silently alter language behavior.

Run relevant tests after moves/splits.

For significant refactors, run the complete available test suite.

---

# 56. Naming

Use consistent names.

Crates:

```text
vut-source
vut-parser
vut-hir
```

Rust modules/files:

```text
snake_case
```

Rust types:

```text
PascalCase
```

Avoid vague files such as:

```text
utils.rs
helpers.rs
misc.rs
common.rs
stuff.rs
```

when a more precise responsibility name exists.

---

# 57. Utility Modules

A utility module is allowed only when its responsibility is coherent.

Instead of:

```text
utils.rs
```

prefer:

```text
path.rs
hash.rs
format.rs
arena.rs
```

depending on actual purpose.

Do not create dumping-ground utility modules.

---

# 58. Platform-Specific Code

When platform behavior differs significantly, isolate it.

Example:

```text
platform/
├── mod.rs
├── windows.rs
├── linux.rs
└── macos.rs
```

Do not fill general compiler modules with large `cfg` blocks when dedicated platform modules are clearer.

---

# 59. Backend-Specific Code

Similarly:

```text
codegen/
├── backend.rs
└── cranelift/
```

Keep Cranelift implementation isolated.

Future LLVM or other backend work should not require rewriting HIR/MIR.

---

# 60. Runtime ABI Separation

Runtime ABI declarations should have a centralized location.

Example:

```text
vut-runtime/src/abi.rs
```

and corresponding backend runtime-call mapping.

Do not duplicate runtime signatures throughout codegen modules.

---

# 61. Error Types

Each crate may define focused internal errors when appropriate.

User-facing compiler errors should still become structured Vut diagnostics.

Do not expose arbitrary internal Rust error strings directly as language diagnostics.

---

# 62. Recommended Current Structure Through Phase 5

Because current implementation has reached approximately Phase 5, the repository should currently need roughly:

```text
crates/
├── vut-source/
├── vut-diagnostics/
├── vut-lexer/
├── vut-ast/
├── vut-parser/
├── vut-symbols/        # if currently needed
├── vut-resolver/
└── vut-compiler/
```

Do not create all Phase 12–19 crates only as placeholders.

---

# 63. Future Structure

As roadmap progresses, add:

```text
Phase 06
→ vut-hir

Phase 07
→ vut-types

Phase 11
→ vut-interface

Phase 12
→ vut-memory
→ vut-mir

Phase 13
→ vut-codegen
→ vut-linker

Phase 14
→ vut-runtime

Phase 15
→ vut-std

Phase 19
→ vut-optimizer
→ vut-incremental
```

---

# 64. Architecture Review After Each Phase

At the end of each roadmap phase, Codex should briefly review:

```text
Did any file become too large?

Does any file now have multiple unrelated responsibilities?

Should any module become a subfolder?

Are there dependency-direction problems?

Did temporary logic leak into the wrong crate?

Is duplicated logic appearing?

Are public APIs broader than necessary?
```

Fix meaningful structural problems before they accumulate.

---

# 65. Do Not Refactor for Aesthetics Only

Do not repeatedly reorganize working code solely because another folder arrangement looks cleaner.

Refactoring should provide concrete benefit such as:

```text
clear responsibility
reduced coupling
better testability
removal of duplication
smaller module complexity
cleaner dependencies
better performance architecture
```

Avoid architecture churn.

---

# 66. Source of Truth

This file defines the preferred project organization.

Subsystem behavior remains defined by the corresponding specification.

Examples:

```text
lexer behavior
→ specs/compiler/lexer.md

parser behavior
→ specs/compiler/parser.md

memory model
→ specs/08-memory-model.md

VPM behavior
→ specs/vpm/
```

This document defines where code should live, not new language semantics.

---

# 67. Final Rules

Codex must follow these principles:

```text
Keep crates focused.

Keep modules focused.

Do not write excessive code into one file.

Split files when responsibilities diverge.

Create subfolders when a subsystem grows.

Create new crates only for real architectural boundaries.

Codex may create files/folders automatically when necessary.

Do not ask permission for routine code organization.

Do not create empty placeholder architecture.

Avoid giant main.rs, lib.rs and mod.rs files.

Avoid circular dependencies.

Keep compiler pipeline layers separated.

Keep backend-specific code isolated.

Keep VPM separate from compiler internals.

Use tests when refactoring.

Prefer maintainability without sacrificing performance.
```

The intended architecture is modular from the beginning and should remain modular as Vut grows.
