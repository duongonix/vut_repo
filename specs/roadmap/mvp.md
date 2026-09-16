# Vut MVP Roadmap

## 1. Purpose

This document defines the implementation roadmap for the Vut MVP.

The roadmap is sequential.

Codex must complete phases in order unless a finalized specification explicitly requires a dependency to be implemented earlier.

The `specs/` directory is the source of truth.

If this roadmap conflicts with a more specific finalized specification, the more specific specification takes precedence.

---

# 2. MVP Goals

The Vut MVP must provide a complete native compilation pipeline:

```text
.vut source
    ↓
lexer
    ↓
parser
    ↓
AST
    ↓
module/name resolution
    ↓
HIR
    ↓
type checking
    ↓
ownership/lifetime analysis
    ↓
MIR
    ↓
optimization
    ↓
native code generation
    ↓
linking
    ↓
native executable
```

The final MVP should support a workflow such as:

```bash
vpm new hello
cd hello
vpm run
```

with a real compiled native executable.

The MVP must not be a VM, interpreter, transpiler or mock compiler.

---

# 3. Core Design Goals

The compiler and runtime must prioritize:

```text
high runtime performance
low memory overhead
fast compilation
strong static typing
predictable behavior
memory safety
native layouts
small runtime
clear diagnostics
simple syntax
modular architecture
```

Performance is a design requirement from Phase 01.

Do not intentionally build a slow temporary architecture with the expectation of replacing it later.

---

# 4. Engineering Rules

Before implementing any phase, read:

```text
specs/24-rules.md
```

Codex must also read the relevant subsystem specifications.

Rules include:

* no giant source files
* no excessive logic in `main.rs`
* separate compiler stages into modules/crates
* VPM remains separate from the compiler core
* use mature existing libraries where appropriate
* avoid unnecessary allocations
* avoid unnecessary cloning
* avoid boxing static values
* do not introduce `dyn` internally as an easy workaround
* do not invent deferred language syntax
* no fake implementations
* no placeholder completion
* no silent fallback behavior

---

# 5. Phase Completion Rules

A phase may be marked `Complete` only when:

```text
implementation exists
architecture matches specs
tests pass
relevant diagnostics exist
no blocking TODO remains
no fake/stub behavior remains
roadmap progress is updated
features.md is updated
```

Each completed phase must update:

```text
specs/roadmap/phases/phase-XX.md
specs/roadmap/features.md
```

---

# 6. Phase Status Format

Each phase file should use:

```markdown
# Phase XX — Name

## Status

Not Started

## Goal

...

## Tasks

- [ ] ...

## Tests

- [ ] ...

## Benchmarks

- [ ] ...

## Completed Work

None.

## Decisions

None.

## Known Limitations

None.
```

Allowed statuses:

```text
Not Started
In Progress
Complete
Blocked
Deferred
```

---

# 7. Phase 01 — Workspace and Compiler Foundation

## Goal

Create the production-quality foundation for the Vut compiler, runtime and tooling.

## Tasks

Create a modular Rust workspace.

Suggested architecture:

```text
crates/
├── vut-source/
├── vut-diagnostics/
├── vut-lexer/
├── vut-parser/
├── vut-ast/
├── vut-hir/
├── vut-types/
├── vut-resolver/
├── vut-ir/
├── vut-optimizer/
├── vut-codegen/
├── vut-runtime/
├── vut-compiler/
├── vut-cli/
└── vpm/
```

Exact crate boundaries may change if a cleaner architecture is justified.

Implement:

```text
CompilerSession
CompilerConfig
SourceManager
SourceId
Span
source loading
UTF-8 validation
line-start index
basic diagnostics infrastructure
compiler error handling
target/build configuration foundation
```

Avoid global mutable compiler state.

## Tests

Test:

```text
source loading
invalid UTF-8 handling
Span correctness
line/column mapping
multiple files
deterministic source IDs
basic diagnostics
```

## Performance

Source lookup must not repeatedly rescan complete files for line/column information.

Maintain line indexes.

---

# 8. Phase 02 — Lexer

## Goal

Implement the complete Vut lexer.

Follow:

```text
specs/compiler/lexer.md
specs/01-language-syntax.md
specs/21-grammar.md
```

## Required Syntax

Support:

```text
identifiers
keywords
numbers
strings
template strings
comments
operators
indentation
newlines
INDENT
DEDENT
@(...)
function calls
ranges
optional marker ?
relative import dots
```

Vut does not support:

```text
[]
{}
anonymous record $()
tuple literals
```

The lexer must no longer contain anonymous-record `$(` logic.

However `$` remains meaningful inside string interpolation.

## Template Strings

Recognize interpolation forms:

```vut
"$name"
"$(expression)"
```

and escaped dollar:

```vut
"\$100"
```

The architecture must preserve precise spans for interpolation diagnostics.

Do not implement runtime string parsing.

## Tests

Test:

```text
indentation
dedentation
blank lines
comments
doc comments
strings
string escapes
template variables
template expressions
invalid template expressions
nested parentheses inside interpolation
@(...) list literals
0..10
0..=10
relative imports
```

Add compile-fail/lexer tests proving `$()` is no longer a valid record literal.

---

# 9. Phase 03 — Parser and AST

## Goal

Implement the parser and syntax AST.

Use:

```text
recursive descent
+
Pratt parser
```

where appropriate.

## Parse

Support:

```text
bindings
type annotations
functions
methods
data
interfaces
enums
type aliases
imports
if
elif
else
match
for
break
continue
return
calls
method calls
field access
list literals
ranges
template strings
optional types
```

Do not implement tuples or anonymous records.

## AST

AST should preserve:

```text
source spans
syntactic structure
template string segments
error/recovery nodes
```

Template strings should conceptually contain:

```text
TemplateString
├── Text
├── Expression
├── Text
└── Expression
```

## Error Recovery

Parser must recover where safely possible.

Do not stop after the first syntax error.

## Tests

Add parser snapshot tests and invalid syntax tests.

---

# 10. Phase 04 — Diagnostics System

## Goal

Implement the full shared diagnostics architecture early.

Follow:

```text
specs/09-errors-diagnostics.md
specs/22-error-codes.md
specs/compiler/diagnostics.md
```

## Required Diagnostic Structure

Diagnostics should support:

```text
severity
error/warning code
title
primary span
secondary spans
source snippet
expected/found
notes
help
related declarations
```

Canonical style:

```text
error[E1003]: type mismatch
  --> src/user.vut:12:9
   |
12 |   age = "20"
   |         ^^^^ expected `int`, found `str`
   |
   = expected: int
   = found:    str
   = help: assign an `int` value to `age`
```

## Rendering

Support:

```text
colored terminal
plain terminal
future machine-readable/IDE representation
```

Compiler stages must emit structured diagnostics rather than ANSI strings.

## Template Diagnostics

Support precise diagnostics for:

```text
invalid interpolation
unclosed $(...)
invalid identifier after $
invalid template expression
invalid escape
```

---

# 11. Phase 05 — Module and Name Resolution

## Goal

Implement modules, imports, scopes, privacy and dependency module roots.

## Modules

Every `.vut` file is a module.

No explicit module declaration.

Project source root:

```text
src/
```

## Imports

Support:

```vut
import math
import math as m
import math at add, plus
```

Relative:

```vut
import .helper
import ..utils
import ...math
```

Meaning:

```text
.     current directory
..    parent
...   two parents
....  three parents
```

## Privacy

Identifiers beginning with `_` are private to their module.

No:

```text
pub
private
export
```

## Cycles

Circular imports are rejected in Vut v1.

Provide clear cycle diagnostics.

## Tests

Test absolute, relative, aliases, selected imports, privacy, conflicts and cycles.

---

# 12. Phase 06 — HIR

## Goal

Create semantic high-level IR.

HIR should normalize resolved program structure while remaining backend-independent.

## IDs

Use compact strongly typed IDs such as:

```text
ModuleId
SymbolId
TypeId
FunctionId
DataId
InterfaceId
EnumId
```

## Responsibilities

HIR should contain:

```text
resolved symbols
resolved imports
normalized methods
implicit self
resolved type references
source origins
semantic expression structure
```

Methods should normalize compiler-injected `self` internally.

Template strings remain typed semantic expressions rather than runtime strings.

---

# 13. Phase 07 — Type System Foundation

## Goal

Implement strict static typing.

## Required Types

Support:

```text
bool

i8 i16 i32 i64
u8 u16 u32 u64

f32 f64

int
float

str
bytes
dyn
null

T?

list(T)
map(K, V)

data types
enum types
function types
interface types
```

Exact low-level ABI for `int` and `float` follows the type-system specification.

## Inference

Example:

```vut
a = 10
```

fixes the static type of `a`.

Later:

```vut
a = "hello"
```

is a compile error.

## dyn

Dynamic behavior is allowed only explicitly:

```vut
value: dyn = 10
```

Do not silently introduce `dyn`.

## Collections

Lists must be homogeneous:

```vut
@(1, 2, 3)
```

Valid.

```vut
@(1, "a", true)
```

Invalid unless explicitly:

```vut
values: list(dyn)
```

## Removed Feature

There is no structural anonymous record type.

`$()` must not exist in the type checker.

---

# 14. Phase 08 — Functions and Methods

## Goal

Implement functions and instance methods.

## Functions

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Support implicit final-expression return.

Explicit `return` supports early exit.

## Methods

Methods use `fn`.

Example:

```vut
fn Counter.increment():
  self.value = self.value + 1
```

`self` is compiler-injected.

Do not allow:

```vut
fn Counter.increment(self: Counter):
```

## Required Features

Support:

```text
parameters
type annotations
named arguments
return checking
implicit final return
method lookup
implicit self
privacy
recursion
```

Static methods and general overloads remain deferred unless another finalized specification defines them.

---

# 15. Phase 09 — Data Model

## Goal

Implement Vut structured data and collections.

## data

Example:

```vut
data User:
  name: str
  age: int
```

Construction:

```vut
user = User(
  name = "Nam",
  age = 20
)
```

Support:

```text
required fields
default fields
private fields
nested data
native layouts
field access
field mutation
```

## No Anonymous Records

The following is invalid:

```vut
config = $(
  host = "localhost"
)
```

Do not add another anonymous-record syntax.

## Lists

Support:

```vut
values = @(1, 2, 3)
```

## Enums

Support basic enum declarations.

Payload-enum syntax remains deferred unless finalized elsewhere.

## Type Aliases

Support:

```vut
type UserId = u64
```

Aliases do not create nominally distinct types.

---

# 16. Phase 10 — Control Flow

## Goal

Implement finalized control-flow features.

## Conditionals

Support:

```vut
if condition:
  ...

elif other:
  ...

else:
  ...
```

Conditions must be `bool`.

No truthiness.

## Expression If

Support:

```vut
value = if condition:
  a
else:
  b
```

Branch result types must satisfy type-checker rules.

## for

Support all four forms:

```vut
for:
  ...

for condition:
  ...

for value in items:
  ...

for value, index in items:
  ...
```

Value comes before index.

## Range

Support:

```vut
0..10
0..=10
```

## Loop Control

Support:

```text
break
continue
```

There is no `while`.

## match

Implement finalized MVP match behavior and exhaustiveness for known finite enums.

---

# 17. Phase 11 — Structural Interfaces

## Goal

Implement Vut structural interfaces.

Example:

```vut
interface Animal:
  speak() -> str
```

A type automatically satisfies an interface when its public methods have matching signatures.

No:

```text
impl
implements
```

## Support

```text
structural satisfaction
public methods only
multiple interfaces
interface composition
interface cycle detection
interface-to-interface compatibility
list(interface)
satisfaction caching
```

Interface runtime layout is separate from semantic compatibility.

---

# 18. Phase 12 — Memory Model and MIR

## Goal

Implement the ownership-aware MIR foundation.

Follow:

```text
specs/08-memory-model.md
specs/compiler/hir.md
specs/compiler/optimization.md
```

Vut memory model is:

```text
value semantics
+
compiler-managed deterministic ownership
+
automatic moves
+
automatic destruction
```

No tracing GC.

No source-level borrow checker.

## MIR

Implement backend-independent MIR with:

```text
basic blocks
values
locals
branches
calls
returns
field access
construction
loops
match lowering
```

Ownership-aware operations must support:

```text
Move
Copy
Drop
Allocate
```

and, only when necessary:

```text
Retain
Release
```

## Type Properties

Compiler should know:

```text
size
alignment
is_copy
needs_drop
contains_managed
ABI classification
```

## Cleanup

Correctly insert cleanup for:

```text
normal scope exit
return
early return
break
error propagation
```

---

# 19. Phase 13 — Native Code Generation

## Goal

Generate real native code.

Primary backend candidate:

```text
Cranelift
```

Do not write a custom machine-code backend for MVP.

## Pipeline

```text
typed HIR
↓
MIR
↓
backend lowering
↓
object file
↓
linker
↓
native executable
```

Never generate backend code directly from AST.

## Support

```text
primitive operations
functions
method calls
locals
branches
comparisons
returns
data layouts
list/runtime calls
string/runtime calls
direct static calls
interface dispatch
dyn support
```

Use native concrete layouts.

Avoid boxing static primitive values.

---

# 20. Phase 14 — Runtime and Memory Management

## Goal

Implement the small native runtime needed by generated programs.

## Runtime Responsibilities

Support:

```text
program startup
heap allocation
heap deallocation
strings
bytes
lists
maps
panic
bounds checks
interface runtime support
dyn runtime support
runtime ABI
basic I/O
```

## Memory Rules

Follow `specs/08-memory-model.md`.

Do not implement a tracing GC.

Do not implement global ARC.

Prefer:

```text
unique ownership
automatic move
deterministic drop
stack/register values
selective RC/COW only when beneficial
```

## Runtime ABI

Runtime calls must have a centralized versioned internal ABI.

Do not scatter backend-specific runtime declarations across codegen.

---

# 21. Phase 15 — Standard Library MVP

## Goal

Implement the MVP standard library.

Required modules:

```text
core
collections
string
io
fs
env
math
time
result-option
```

Follow:

```text
specs/std/
```

## Console I/O

Official basic console functions:

```text
print()
out()
input()
```

### print

```vut
print("Hello")
```

does not append a newline.

### out

```vut
out("Hello")
```

appends a newline.

### input

```vut
name = input("Name: ")
```

returns:

```text
str
```

The prompt does not automatically append a newline.

`input()` should flush stdout as required before reading.

Do not introduce `println` as another basic output API.

## Template Strings

Both:

```text
print()
out()
```

must support statically compiled template strings.

Examples:

```vut
out("Hello $name")
out("Birth year: $(year - age)")
```

No runtime `eval`.

No runtime parsing of Vut expressions.

## Implementation

Use mature Rust/platform libraries for:

```text
Unicode
filesystem
I/O
time
hash maps
math
```

where appropriate.

Do not unnecessarily reimplement mature infrastructure.

---

# 22. Phase 16 — Vut CLI

## Goal

Implement the official compiler CLI.

`vut` has only two main commands:

```bash
vut run
vut build
```

Examples:

```bash
vut run
vut run hello.vut

vut build
vut build hello.vut
```

Support relevant flags such as:

```text
--release
--target
--output
--help
--version
```

Flags are not separate commands.

## Program Arguments

Support:

```bash
vut run -- arg1 arg2
```

The CLI should remain thin.

Compiler logic belongs in compiler crates.

---

# 23. Phase 17 — VPM Core

## Goal

Implement Vut project and package workflow.

Core commands:

```text
vpm new
vpm init

vpm add
vpm remove
vpm install
vpm update

vpm build
vpm run
vpm check
```

## Project

Default project:

```text
project/
├── src/
│   └── main.vut
├── tests/
├── vpm.toml
└── vpm.lock
```

## VPM Architecture

VPM must reuse compiler APIs.

Do not duplicate compiler functionality.

Implement:

```text
manifest parsing
lockfile foundation
dependency model
local package store
downloads cache
build cache foundation
atomic file writes
```

---

# 24. Phase 18 — Package Resolution and Providers

## Goal

Implement real package resolution.

## Default Registry

Default registry:

```text
github.com/duongonix/vpm
```

Package structure:

```text
math/
├── v0.1.0/
├── v0.1.1/
└── v1.0.0/
```

## Registry Commands

```bash
vpm add math
vpm add math@latest
vpm add math@1.0.0
```

`vpm add math` means latest stable.

## Self-Hosted GitHub

Minimum:

```bash
vpm add owner/repo/package
```

Example:

```bash
vpm add nam/abc/math@1.2.0
```

A repository root can never itself be a package.

## GitLab

Support explicit provider:

```bash
vpm add gitlab:nam/abc/math@1.2.0
```

## SemVer

Use mature SemVer parsing.

Stable latest excludes prereleases.

## Conflicts

VPM v1 uses one package namespace/version per dependency graph.

If incompatible dependency versions are required:

```text
reject dependency graph
```

Do not silently load multiple versions under hidden aliases.

---

# 25. Phase 19 — Optimization and Incremental Compilation

## Goal

Implement serious release optimization and reusable compilation caches.

## Optimization

Support where practical:

```text
constant folding
constant propagation
dead-code elimination
dead-branch elimination
copy elision
drop elimination
last-use move optimization
escape analysis
stack promotion
scalar replacement
allocation elimination
bounds-check elimination
interface devirtualization
retain/release elimination
basic inlining
CSE
simplification
```

## Memory Optimization

Follow `specs/08-memory-model.md`.

Measure:

```text
allocation count
peak memory
copy count
move count
retain/release count
execution time
```

where practical.

## Incremental Compilation

Implement:

```text
source fingerprinting
module dependency graph
cache keys
cache schema version
target-aware cache
debug/release cache separation
package build cache
object cache
```

Use BLAKE3 or another approved mature content hash.

Corrupt cache entries must be safely discarded and rebuilt.

No-change builds should perform minimal work.

---

# 26. Phase 20 — Tooling, Testing and MVP Hardening

## Goal

Complete the MVP as a usable language toolchain.

## VPM Tooling

Implement finalized commands where their specifications are complete:

```text
vpm test
vpm fmt
vpm lint
vpm doc
vpm clean
vpm tree
vpm outdated
vpm search
vpm info
```

Publishing/authentication commands remain unavailable until their semantics are finalized:

```text
vpm login
vpm publish
vpm yank
```

unless a later finalized specification defines them.

## Formatter

Canonical indentation:

```text
2 spaces
```

## Tests

The final MVP requires:

```text
lexer tests
parser tests
AST tests
resolver tests
type-check tests
interface tests
HIR tests
MIR tests
ownership tests
memory-safety tests
codegen tests
runtime tests
stdlib tests
CLI tests
VPM tests
package-resolution tests
formatter tests
linter tests
integration tests
compile-pass tests
compile-fail tests
diagnostic snapshot tests
fuzz tests where appropriate
benchmarks
```

## Required Language Integration Test

At minimum verify a program using:

```vut
data User:
  name: str
  age: int

fn User.greet():
  out("Hello $self.name")

fn main():
  name = input("Name: ")

  user = User(
    name = name,
    age = 20
  )

  user.greet()

  values = @(10, 20, 30)

  for value, index in values:
    out("$index: $value")
```

compiles and executes correctly.

---

# 27. Explicitly Deferred Features

The following are not required for MVP unless later finalized specs override this roadmap:

```text
full concurrency
thread-sharing model
advanced FFI
complete unsafe API
full generic declaration syntax
generic constraints
payload enum final syntax
extension methods
static methods
general function overloading
reflection
JIT
VM
REPL
LSP
debugger
package publishing
package authentication
package yanking
explicit reference syntax
source-level borrow checker
general user-defined destructors
```

`async fn` and `await` are now specified for a single-thread foundation by
`specs/async/`, as a post-MVP feature. Full concurrency remains deferred.

Do not invent these features while implementing MVP phases.

---

# 28. Removed Features

The following must not be implemented:

```text
anonymous record $()
tuple type
tuple literal
while
class
inheritance
impl
implements
pub
private keyword
export keyword
is operator
global tracing GC
println as basic Vut output API
```

If older code or documentation contains these features, update or remove it.

---

# 29. Final MVP Definition of Done

Vut MVP is complete when all required phases are complete and this workflow succeeds:

```bash
vpm new hello
cd hello
vpm run
```

The command must:

```text
read project
resolve dependencies
resolve modules
lex source
parse source
build AST
resolve symbols
lower HIR
type check
analyze ownership
build MIR
optimize
generate native object code
link executable
run executable
```

with real native compilation.

The toolchain must also correctly support:

```text
strict static typing
data
functions
methods
interfaces
if/match/for
lists
maps
strings
template strings
print
out
input
modules
packages
automatic deterministic memory management
native code generation
clear diagnostics
```

---

# 30. Codex Execution Rule

When Codex begins work:

```text
1. read all finalized specs
2. inspect current codebase
3. identify current roadmap phase
4. implement only according to finalized behavior
5. create/modularize files as needed
6. use mature dependencies where appropriate
7. run tests
8. run formatter/linter
9. update phase file
10. update features.md
11. continue to next phase only after current phase is truly complete
```

Do not mark work complete merely because code compiles.

Correctness, tests, architecture, memory behavior and spec compliance are all required.
