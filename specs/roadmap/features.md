# Vut Feature Status

## 1. Purpose

This file tracks implementation status for Vut features.

It must be updated after every roadmap phase.

This file is a progress tracker, not a replacement for the detailed specifications.

Detailed behavior is defined by:

```text
specs/*.md
specs/compiler/*.md
specs/std/*.md
specs/vpm/*.md
```

---

# 2. Status Values

Use only:

```text
Not Started
In Progress
Complete
Blocked
Deferred
Removed
```

Meaning:

### Not Started

Feature is specified but implementation has not started.

### In Progress

Implementation exists but feature is not yet fully complete or tested.

### Complete

Feature is implemented, tested and compliant with finalized specs.

### Blocked

Implementation cannot continue because another required component is incomplete.

### Deferred

Feature is intentionally outside the MVP.

### Removed

Feature has been explicitly rejected from the language/toolchain.

---

# 3. Compiler Foundation

| Feature                                 | Status      | Phase |
| --------------------------------------- | ----------- | ----: |
| Rust workspace                          | Complete    |    01 |
| CompilerSession                         | Complete    |    01 |
| Compiler configuration                  | Complete    |    01 |
| SourceManager                           | Complete    |    01 |
| SourceId                                | Complete    |    01 |
| Span                                    | Complete    |    01 |
| UTF-8 source loading                    | Complete    |    01 |
| Line index                              | Complete    |    01 |
| Target configuration foundation         | Complete    |    01 |
| Build mode foundation                   | Complete    |    01 |
| Structured compiler errors              | Complete    |    01 |
| Global-state-free compiler architecture | Complete    |    01 |

---

# 4. Lexer

| Feature                         | Status      | Phase |
| ------------------------------- | ----------- | ----: |
| Identifier lexing               | Complete    |    02 |
| Keyword lexing                  | Complete    |    02 |
| Integer literals                | Complete    |    02 |
| Float literals                  | Complete    |    02 |
| String literals                 | Complete    |    02 |
| String escapes                  | Complete    |    02 |
| Template string lexing          | Complete    |    02 |
| `$identifier` interpolation     | Complete    |    02 |
| `$(expression)` interpolation   | Complete    |    02 |
| Escaped `$`                     | Complete    |    02 |
| Single-line comments `#`        | Complete    |    02 |
| Multi-line comments `## ... ##` | Complete    |    02 |
| Doc comments `###`              | Complete    |    02 |
| Significant newlines            | Complete    |    02 |
| INDENT                          | Complete    |    02 |
| DEDENT                          | Complete    |    02 |
| Parentheses                     | Complete    |    02 |
| `@(...)` list literals          | Complete    |    02 |
| Range `..`                      | Complete    |    02 |
| Inclusive range `..=`           | Complete    |    02 |
| Relative-import dots            | Complete    |    02 |
| Lexer recovery                  | Complete    |    02 |
| Precise interpolation spans     | Complete    |    02 |
| Anonymous-record `$()` lexing   | Removed     |     — |

---

# 5. Parser

| Feature                  | Status      | Phase |
| ------------------------ | ----------- | ----: |
| Recursive-descent parser | Complete    |    03 |
| Pratt expression parser  | Complete    |    03 |
| Bindings                 | Complete    |    03 |
| Type annotations         | Complete    |    03 |
| Function declarations    | Complete    |    03 |
| Method declarations      | Complete    |    03 |
| Data declarations        | Complete    |    03 |
| Interface declarations   | Complete    |    03 |
| Enum declarations        | Complete    |    03 |
| Type aliases             | Complete    |    03 |
| Imports                  | Complete    |    03 |
| If statements            | Complete    |    03 |
| If expressions           | Complete    |    03 |
| Match                    | Complete    |    03 |
| For loops                | Complete    |    03 |
| Break                    | Complete    |    03 |
| Continue                 | Complete    |    03 |
| Return                   | Complete    |    03 |
| Calls                    | Complete    |    03 |
| Named arguments          | Complete    |    03 |
| Method calls             | Complete    |    03 |
| Field access             | Complete    |    03 |
| List literals            | Complete    |    03 |
| Range expressions        | Complete    |    03 |
| Template-string AST      | Complete    |    03 |
| Parser recovery          | Complete    |    03 |
| Anonymous record parser  | Removed     |     — |
| Tuple parser             | Removed     |     — |

---

# 6. Diagnostics

| Feature                                | Status      | Phase |
| -------------------------------------- | ----------- | ----: |
| Structured Diagnostic type             | Complete    |    04 |
| Error codes                            | Complete    |    04 |
| Warning codes                          | Complete    |    04 |
| Primary spans                          | Complete    |    04 |
| Secondary spans                        | Complete    |    04 |
| Source frames                          | Complete    |    04 |
| Expected/found output                  | Complete    |    04 |
| Notes                                  | Complete    |    04 |
| Help messages                          | Complete    |    04 |
| Related declaration locations          | Complete    |    04 |
| Colored terminal rendering             | Complete    |    04 |
| Plain rendering                        | Complete    |    04 |
| Deterministic diagnostic ordering      | Complete    |    04 |
| Multiple-error recovery                | Complete    |    04 |
| Cascading-error suppression            | Complete    |    04 |
| Invalid interpolation diagnostic       | Complete    |    04 |
| Unclosed interpolation diagnostic      | Complete    |    04 |
| Invalid template expression diagnostic | Complete    |    04 |
| Invalid escape diagnostic              | Complete    |    04 |

Diagnostic-code status (M1.1 audit, `specs/22`): `E4001` (unknown interface
parent, resolver) and `E4004` (invalid interface parent, checker) are emitted.
`E1006`/`E1007`/`E1104` are emitted. `E1002`, `E1011`, `E4002`, `E4003`,
`E5008`, and `E5010` currently have no reachable compiler scenario and are
reserved; they must not be emitted speculatively. `E1105` (mutation through a
constant binding) is blocked on const-depth semantics in `specs/08`.

---

# 7. Modules and Name Resolution

| Feature                      | Status      | Phase |
| ---------------------------- | ----------- | ----: |
| `.vut` file modules          | Complete    |    05 |
| `mod.vut` folder modules     | Complete    | current |
| File-over-`mod.vut` precedence | Complete  | current |
| Source-root module discovery | Complete    |    05 |
| Absolute imports             | Complete    |    05 |
| Import aliases               | Complete    |    05 |
| Selected `at` imports        | Complete    |    05 |
| `.name` imports              | Complete    |    05 |
| `..name` imports             | Complete    |    05 |
| `...name` imports            | Complete    |    05 |
| Module graph                 | Complete    |    05 |
| Circular-import detection    | Complete    |    05 |
| Module symbol tables         | Complete    |    05 |
| Function scope               | Complete    |    05 |
| Block scope                  | Complete    |    05 |
| Private `_name` symbols      | Complete    |    05 |
| Import collision diagnostics | Complete    |    05 |
| Local/package namespace conflict | Complete | current |
| Root-escape protection       | Complete    |    05 |

---

# 8. HIR

| Feature                    | Status      | Phase |
| -------------------------- | ----------- | ----: |
| HirId                      | Complete    |    06 |
| SymbolId                   | Complete    |    06 |
| ModuleId                   | Complete    |    06 |
| TypeId                     | Complete    |    06 |
| FunctionId                 | Complete    |    06 |
| DataId                     | Complete    |    06 |
| InterfaceId                | Complete    |    06 |
| EnumId                     | Complete    |    06 |
| Resolved symbol references | Complete    |    06 |
| Resolved imports           | Complete    |    06 |
| Method normalization       | Complete    |    06 |
| Compiler-injected `self`   | Complete    |    06 |
| HIR source spans           | Complete    |    06 |
| HIR side tables            | Partial     |    06 |
| HIR template strings       | Partial     |    06 |

---

# 8A. Attributes

| Feature                            | Status      | Phase   |
| ---------------------------------- | ----------- | ------: |
| Attribute syntax `@name`           | Complete    | current |
| Attribute arguments                | Complete    | current |
| List literal `@(...)` compatibility | Complete   | current |
| AST attribute representation       | Complete    | current |
| HIR/semantic attribute metadata    | Complete    | current |
| Built-in attribute registry        | Complete    | current |
| Unknown attribute diagnostics      | Complete    | current |
| Duplicate attribute diagnostics    | Complete    | current |
| Invalid target diagnostics         | Complete    | current |
| `@repr(C)`                         | Complete    | current |
| `@repr(transparent)`               | Complete    | current |
| `@link_name("...")`                | Complete    | current |

---

# 9. Primitive Type System

| Feature                      | Status      | Phase |
| ---------------------------- | ----------- | ----: |
| `bool`                       | Complete    |    07 |
| `i8`                         | Complete    |    07 |
| `i16`                        | Complete    |    07 |
| `i32`                        | Complete    |    07 |
| `i64`                        | Complete    |    07 |
| `u8`                         | Complete    |    07 |
| `u16`                        | Complete    |    07 |
| `u32`                        | Complete    |    07 |
| `u64`                        | Complete    |    07 |
| `usize`                      | Complete    | current |
| `isize`                      | Complete    | current |
| `f32`                        | Complete    |    07 |
| `f64`                        | Complete    |    07 |
| `int`                        | Complete    |    07 |
| `float`                      | Complete    |    07 |
| `str`                        | Complete    |    07 |
| `bytes`                      | Complete    |    07 |
| Native `Utf8Error` type      | Complete    | current |
| `bytes` method surface       | Complete    | current |
| `bytes` conversions          | Complete    | current |
| `bytes` bounds safety        | Complete    | current |
| `bytes` ownership/COW/drop   | Complete    | current |
| Native `HexError` type       | Complete    | current |
| `bytes` buffer ops (push/extend/truncate/resize) | Complete | current |
| `bytes.find` / `starts_with` / `ends_with` / `compare` | Complete | current |
| `bytes.to_hex` / `bytes.from_hex` (HexError) | Complete | current |
| Explicit-endian bytes read/write (`_le`/`_be`) | Complete | current |
| `dyn`                        | In Progress |    07 |
| `null`                       | Complete    |    07 |
| Optional `T?`                | Complete    |    07 |
| Compiler internal error type | Complete    |    07 |

---

# 10. Type Inference and Checking

| Feature                         | Status      | Phase |
| ------------------------------- | ----------- | ----: |
| First-assignment type inference | Complete    |    07 |
| Fixed variable type             | Complete    |    07 |
| Explicit annotations            | Complete    |    07 |
| Strict assignments              | Complete    |    07 |
| No implicit `dyn`               | Complete    |    07 |
| Numeric operator checking       | Complete    |    07 |
| Comparison checking             | Complete    |    07 |
| `and`                           | Complete    |    07 |
| `or`                            | Complete    |    07 |
| `not`                           | Complete    |    07 |
| Bool-only conditions            | Complete    |    07 |
| No truthiness                   | Complete    |    07 |
| Optional/null checking          | Complete    |    07 |
| Function-call checking          | Complete    |    08 |
| Return checking                 | Complete    |    08 |
| Method-call checking            | Complete    |    08 |
| Named-argument checking         | Complete    |    08 |
| Data-constructor checking       | Complete    |    09 |
| If-expression typing            | Complete    |    10 |
| Match-expression typing         | Complete    |    10 |

---

# 11. Collections

| Feature                      | Status      | Phase |
| ---------------------------- | ----------- | ----: |
| `list(T)` type               | Complete    |    07 |
| Homogeneous list enforcement | Complete    |    07 |
| Explicit `list(dyn)`         | Complete    |    07 |
| Empty-list contextual typing | Complete    |    07 |
| `@(...)` list literal        | Complete    |    09 |
| `map(K, V)` type             | Complete    |    07 |
| `map((K, V), ...)` literal   | Complete    | current |
| Map runtime                  | Complete    |    14 |
| List runtime utility API     | Complete    |    14 |
| Compiler-managed list ABI    | Complete    | current |
| Native list element layouts  | Complete    | current |
| `array(T, N)` type           | Complete    |  current |
| `array(...)` literal         | Complete    |  current |
| Inline native array layout   | Complete    |  current |
| Array len/at/set/fill        | Complete    |  current |
| Array first/last             | Complete    |  current |
| Array iteration              | Complete    |  current |
| Array `to_list()`            | Complete    |  current |
| Array `contains()`           | Complete    |  current |
| Array `reverse()` / `sort()` | Complete    |  current |
| Managed/nested array cleanup | Complete    |  current |
| `.len()`                     | Complete    | 15/20 |
| `.is_empty()`                | Complete    |    20 |
| `.capacity()`                | Complete    | current |
| `.push()`                    | Complete    | current |
| `.pop()`                     | Complete    | current |
| `.first()` / `.last()`       | Complete    | current |
| `.index_of()`                | Complete    | current |
| `.extend()`                  | Complete    | current |
| `.reverse()`                 | Complete    | current |
| `.sort()` (int/float/str/bool) | Complete  | current |
| `.truncate()`                | Complete    | current |
| `.swap()`                    | Complete    | current |
| `.shrink_to_fit()`           | Complete    | current |
| `.map()` / `.filter()` (compiler-lowered) | Complete | current |
| `.fold()` (compiler-lowered) | Complete | current |
| `.any()` / `.all()` (compiler-lowered, short-circuit) | Complete | current |
| `.find_index()` (compiler-lowered, short-circuit) | Complete | current |
| `.sort_by()` (compiler-lowered, stable) | Complete | current |
| `.join()` (`list(str)`, native runtime) | Complete | current |
| `.at()`                      | Complete    | current |
| `.set()`                     | Complete    | current |
| `.slice()`                   | Complete    | current |
| `.insert()`                  | Complete    | current |
| `.remove()`                  | Complete    | current |
| `.clear()`                   | Complete    | current |
| `.contains()`                | Complete    | current |
| Map get/set/remove           | Complete    | current |
| Map contains-key/len/clear   | Complete    | current |
| Map capacity/reserve         | Complete    | current |
| Map `keys()` (all key types) | Complete    | current |
| Map `values()`               | Complete    | current |
| Map `get_or()`               | Complete    | current |

---

# 12. Data Model

| Feature                      | Status      | Phase |
| ---------------------------- | ----------- | ----: |
| `data` declaration           | Complete    |    09 |
| Named data construction      | Complete    |    09 |
| Required fields              | Complete    |    09 |
| Default fields               | Complete    |    09 |
| Nested data                  | Complete    |    09 |
| Private fields               | Complete    |    09 |
| Mutable fields               | Complete    |    09 |
| Native data layout           | Complete    |    09 |
| Field access                 | Complete    |    09 |
| Field mutation               | Complete    |    09 |
| Basic enums                  | Complete    |    09 |
| Type aliases                 | Complete    |    09 |
| Anonymous structural records | Removed     |     — |
| Tuple type                   | Removed     |     — |
| Tuple literal                | Removed     |     — |

---

# 13. Functions

| Feature                          | Status      | Phase |
| -------------------------------- | ----------- | ----: |
| Free functions                   | Complete    |    08 |
| Typed parameters                 | Complete    |    08 |
| Inferred local types             | Complete    |    08 |
| Explicit return type             | Complete    |    08 |
| `void` return type               | Complete    |    08 |
| Omitted return defaults to void  | Complete    |    08 |
| Void `main` maps to exit code 0  | Complete    | 13/16 |
| Implicit final-expression return | Complete    |    08 |
| Explicit early `return`          | Complete    |    08 |
| Named arguments                  | Complete    |    08 |
| Recursion                        | Complete    |    08 |
| General overloading              | Deferred    |     — |
| Non-capturing lambdas            | Complete    |     — |
| Capturing closures               | Deferred    |     — |

## Receiver Functions

Defined by `specs/receiver/`. Receiver functions are a general language feature,
not a DSL or Vutcom subsystem (`specs/vutcom/` is superseded by `specs/receiver/`).

| Feature                                             | Status      | Phase |
| --------------------------------------------------- | ----------- | ----: |
| Receiver function type `fn(R)(Args...) -> T`        | Complete    |     — |
| Receiver kept distinct from ordinary parameters      | Complete    |     — |
| Type compatibility by receiver                      | Complete    |     — |
| Trailing body `Call():`                             | Complete    |     — |
| Trailing body `Call(args) (params):`                | Complete    |     — |
| `.call(receiver, ...args)` invocation               | Complete    |     — |
| `.call()` reserved for receiver functions (`E1025`) | Complete    |     — |
| Implicit receiver method lookup                     | Complete    |     — |
| Nested receiver scopes                              | Complete    |     — |
| Receiver diagnostics (`E1020`–`E1026`)              | Complete    |     — |
| Non-escaping zero-heap receiver closures            | Complete    |     — |
| Capturing receiver closures                         | Deferred    |     — |

Receiver functions are non-capturing for now; a receiver body that references an
enclosing local still reports `E1013`. Capturing closures use the generic closure
escape/ownership rules when implemented, with no receiver-specific box.

---

# 14. Methods

| Feature                        | Status      | Phase |
| ------------------------------ | ----------- | ----: |
| `fn Type.method()` declaration | Complete    |    03 |
| Required `fn` on methods       | Complete    |    03 |
| Implicit `self`                | Complete    |    05 |
| Method namespace resolution    | Complete    |    05 |
| Method-call type checking      | Complete    |    08 |
| Method privacy                 | Complete    |    08 |
| Generated `Type()` constructor | Complete    |    09 |
| Static methods                 | Complete    |  08/11 |
| Extension methods (same module) | Complete   |    05 |
| Extension methods (cross module)| Deferred   |     - |
---

# 15. Control Flow

| Feature                       | Status      | Phase |
| ----------------------------- | ----------- | ----: |
| `if`                          | Complete    |    10 |
| `elif`                        | Complete    |    10 |
| `else`                        | Complete    |    10 |
| If expressions                | Complete    |    10 |
| `match`                       | Complete    |    10 |
| Enum exhaustiveness           | Complete    |    10 |
| Infinite `for:`               | Complete    |    10 |
| Conditional `for condition:`  | Complete    |    10 |
| Iterable `for value in`       | Complete    |    10 |
| Indexed `for value, index in` | Complete    |    10 |
| Exclusive range `..`          | Complete    |    10 |
| Inclusive range `..=`         | Complete    |    10 |
| `break`                       | Complete    |    10 |
| `continue`                    | Complete    |    10 |
| `while`                       | Removed     |     — |

---

# 16. Interfaces

| Feature                              | Status      | Phase |
| ------------------------------------ | ----------- | ----: |
| Interface declarations               | Complete    |    11 |
| Structural satisfaction              | Complete    |    11 |
| Automatic satisfaction               | Complete    |    11 |
| Public-method-only satisfaction      | Complete    |    11 |
| Multiple interfaces                  | Complete    |    11 |
| Interface composition                | Complete    |    11 |
| Composition conflict detection       | Complete    |    11 |
| Interface cycle detection            | Complete    |    11 |
| Interface-to-interface compatibility | Complete    |    11 |
| Satisfaction cache                   | Complete    |    11 |
| `list(Interface)`                    | Complete    |    11 |
| `impl`                               | Removed     |     — |
| `implements`                         | Removed     |     — |
| Class inheritance                    | Removed     |     — |

---

# 17. Memory Model

| Feature                        | Status      |  Phase |
| ------------------------------ | ----------- | -----: |
| Value semantics                | Complete    |     12 |
| Compiler-managed ownership     | Complete    |     12 |
| Automatic moves                | Complete    |     12 |
| Automatic drops                | Complete    |     12 |
| Deterministic destruction      | Complete    |     12 |
| Copy-type classification       | Complete    |     12 |
| `needs_drop` type property     | Complete    |     12 |
| Managed-type classification    | Complete    |     12 |
| Type layout metadata           | Foundation  |     12 |
| Last-use analysis              | Complete    |     12 |
| Cleanup insertion              | Complete    |     12 |
| Early-return cleanup           | Complete    |     12 |
| Break cleanup                  | Complete    |     12 |
| Error-propagation cleanup      | Complete    |     12 |
| No tracing GC                  | Complete    | Design |
| No source-level borrow checker | Complete    | Design |
| No manual free in safe Vut     | Complete    | Design |
| Global ARC                     | Removed     |      — |
| Selective RC support           | Complete    |  14/19 |
| Selective COW support          | Complete    |  14/19 |

---

# 18. MIR

| Feature                 | Status      | Phase |
| ----------------------- | ----------- | ----: |
| MIR functions           | Complete    |    12 |
| Basic blocks            | Complete    |    12 |
| MIR values              | Complete    |    12 |
| MIR locals              | Complete    |    12 |
| Branching               | Complete    |    12 |
| Calls                   | Complete    |    12 |
| Returns                 | Complete    |    12 |
| Field access            | Complete    |    12 |
| Data construction       | Complete    |    12 |
| Loop lowering           | Complete    |    12 |
| Match lowering          | Complete    |    12 |
| `Move`                  | Complete    |    12 |
| `Copy`                  | Complete    |    12 |
| `Drop`                  | Complete    |    12 |
| `Allocate`              | Complete    |    12 |
| `Retain` when required  | Complete    | 14/19 |
| `Release` when required | Complete    | 14/19 |

---

# 19. Native Code Generation

| Feature             | Status      | Phase |
| ------------------- | ----------- | ----: |
| Backend abstraction | Complete    |    13 |
| Cranelift backend   | Complete    |    13 |
| MIR lowering        | Complete    |    13 |
| Primitive lowering  | Complete    |    13 |
| Function codegen    | Complete    |    13 |
| Method codegen      | Complete    |    13 |
| Direct static calls | Complete    |    13 |
| Branch codegen      | Complete    |    13 |
| Arithmetic          | Complete    |    13 |
| Comparisons         | Complete    |    13 |
| Return ABI          | Complete    |    13 |
| Data layouts        | Complete    |    13 |
| Field offsets       | Complete    |    13 |
| Object emission     | Complete    |    13 |
| Native linking      | Complete    |    13 |
| Target abstraction  | Complete    |    13 |

---

# 20. Runtime

| Feature                          | Status      | Phase |
| -------------------------------- | ----------- | ----: |
| Runtime startup                  | Complete    |    14 |
| Runtime ABI                      | Complete    |    14 |
| Heap allocation                  | Complete    |    14 |
| Heap deallocation                | Complete    |    14 |
| Strings                          | Complete    |    14 |
| Bytes                            | Complete    |    14 |
| Bytes contiguous buffer          | Complete    | current |
| Bytes UTF-8 validation           | Complete    | current |
| Managed leak accounting          | Complete    | current |
| Managed array cleanup            | Complete    | current |
| Builtin method tooling (LSP)     | Complete    | current |
| Lists                            | Complete    |    14 |
| Maps                             | Complete    |    14 |
| Panic runtime                    | Complete    |    14 |
| Bounds-check failure             | Complete    |    14 |
| Interface runtime representation | Complete    |    14 |
| Dyn runtime representation       | In Progress |    14 |
| Basic I/O bridge                 | Complete    |    14 |
| Tracing GC                       | Removed     |     — |

---

# 21. Strings and Template Strings

| Feature                               | Status      |    Phase |
| ------------------------------------- | ----------- | -------: |
| UTF-8 `str`                           | Complete    |    14/15 |
| Static string literals (rodata)       | Complete    |    13/14 |
| Zero-alloc string literals            | Not Started |  current |
| String length APIs                    | Complete    |       15 |
| Byte length                           | Complete    |       15 |
| Character length                      | Complete    |       15 |
| Contains                              | Complete    |       15 |
| Starts-with                           | Complete    |       15 |
| Ends-with                             | Complete    |       15 |
| Trim                                  | Complete    |       15 |
| Case conversion                       | Complete    |       15 |
| Replace                               | Complete    |       15 |
| Split                                 | Complete    |    current |
| Lines                                 | Complete    |    current |
| Split whitespace                      | Complete    |    current |
| `chars` / `char_at` (Unicode scalar)  | Complete    |    current |
| `repeat` / `pad_left` / `pad_right`   | Complete    |    current |
| `strip_prefix` / `strip_suffix`       | Complete    |    current |
| `rfind` / `compare`                   | Complete    |    current |
| `to_int` / `to_float` aliases         | Complete    |    current |
| Numeric `.to_str()`                   | Complete     |       20 |
| Numeric `abs`/`min`/`max`/`clamp`     | Complete     |  current |
| `int.pow` / `float.pow`               | Complete     |  current |
| Float `floor`/`ceil`/`round`/`trunc`/`sqrt` | Complete | current |
| `float.to_int` / `is_nan` / `is_finite` | Complete   |  current |
| `bool.to_str`                         | Complete     |  current |
| `result.is_ok` / `is_err` / `unwrap_or` | Complete   |  current |
| Template strings                      | Complete    | 02/03/15 |
| `$identifier` formatting              | Complete    |       15 |
| `$(expression)` formatting            | Complete    |       15 |
| Static type checking of interpolation | Complete    |    07/15 |
| Runtime-free expression parsing       | Complete    |    02/03 |
| Direct buffered template output       | Foundation  |    15/19 |

---

# 22. Standard I/O

| Feature                       | Status      | Phase |
| ----------------------------- | ----------- | ----: |
| `print()`                     | Complete    |    15 |
| `print()` without newline     | Complete    |    15 |
| `out()`                       | Complete    |    15 |
| `out()` with newline          | Complete    |    15 |
| `input(str) -> str`           | Complete    |    15 |
| Input prompt without newline  | Complete    |    15 |
| Prompt flushing               | Complete    |    15 |
| Input line-ending removal     | Complete    |    15 |
| Template support in `print()` | Complete    |    15 |
| Template support in `out()`   | Complete    |    15 |
| Direct display of any value   | Complete    |     — |
| Multi-argument `print`/`out`  | Complete    |     — |
| Zero-argument `print`/`out`   | Complete    |     — |
| Interpolation of aggregates   | Complete    |     — |
| `println()` basic API         | Removed     |     — |

---

# 23. Standard Library

| Feature                 | Status      | Phase |
| ----------------------- | ----------- | ----: |
| `std.core`              | Complete    |    15 |
| `std.collections`       | Removed (builtin `list` methods) | current |
| `std.string`            | Complete    |    15 |
| `std.io`                | Complete    |    15 |
| `std.fs`                | Complete    |    15 |
| `std.env`               | Complete    |    15 |
| `std.math`              | Not Started |     - |
| `std.path`              | Complete    |    15 |
| `std.os`                | Complete    |    15 |
| `std.process`           | Complete    |    15 |
| `std.http`              | Complete    | current |
| `std.json`              | Complete    | current |
| `std.time`              | Complete    |    15 |
| Optional support        | Complete    |    15 |
| Result support          | Complete    |    15 |
| Numeric conversions     | Complete    |    15 |
| String conversions      | Complete    |    15 |
| Filesystem typed errors | Complete    |    15 |
| Environment APIs        | Complete    |    15 |
| Time Duration           | Complete    |    15 |
| Monotonic Instant       | Complete    |    15 |

---

# 24. Error Handling

| Feature                                | Status      |  Phase |
| -------------------------------------- | ----------- | -----: |
| Result-style error handling            | Complete    |     15 |
| Optional absence                       | Complete    |  07/15 |
| `?` propagation                        | Complete    |  10/15 |
| No exception-driven core model         | Complete    | Design |
| Panic for programmer/invariant failure | Complete    |  14/15 |
| Payload enum final syntax              | Complete    |  09/10 |

---

# 25. Vut CLI

| Feature                 | Status      | Phase |
| ----------------------- | ----------- | ----: |
| `vut run`               | Complete    |    16 |
| `vut build`             | Complete    |    16 |
| Run `.vut` file         | Complete    |    16 |
| Build `.vut` file       | Complete    |    16 |
| Run project             | Complete    |    16 |
| Build project           | Complete    |    16 |
| `--release`             | Complete    |    16 |
| `--target`              | Complete    |    16 |
| `--output`              | Complete    |    16 |
| `--help`                | Complete    |    16 |
| `--version`             | Complete    |    16 |
| Program args after `--` | Complete    |    16 |

---

# 26. VPM Project Commands

| Feature       | Status      | Phase |
| ------------- | ----------- | ----: |
| `vpm new`     | Complete    |    17 |
| `vpm init`    | Complete    |    17 |
| `vpm add`     | Complete    | 17/18 |
| `vpm remove`  | Complete    |    17 |
| `vpm install` | Complete    |    17 |
| `vpm update`  | Complete    | 17/18 |
| `vpm build`   | Complete    |    17 |
| `vpm run`     | Complete    |    17 |
| `vpm check`   | Complete    |    17 |

---

# 27. VPM Package System

| Feature                               | Status      | Phase |
| ------------------------------------- | ----------- | ----: |
| `vpm.toml`                            | Complete    |    17 |
| `vpm.lock`                            | Complete    |    17 |
| Global package store                  | Complete    |    17 |
| Download cache                        | Foundation  |    17 |
| Build cache                           | Foundation  | 17/19 |
| Source-only packages                  | Complete    |    18 |
| `.vutlib`                             | Removed     |     — |
| Package version folders               | Complete    |    18 |
| `v<semver>` resolution                | Complete    |    18 |
| Default registry provider             | Complete    |    18 |
| GitHub provider                       | Complete    |    18 |
| GitLab provider                       | Complete    |    18 |
| Stable `latest` resolution            | Complete    |    18 |
| Explicit prerelease install           | Complete    |    18 |
| Manifest/path validation              | Complete    |    18 |
| Checksums/revisions                   | Complete    |    18 |
| Package immutability checking         | Complete    |    18 |
| Strict manifest schema validation      | Complete    | 18/20 |
| Versioned lockfile compatibility       | Complete    | 17/20 |
| Atomic pre-install package validation  | Complete    | 18/20 |
| Package-name collision detection      | Complete    |    18 |
| Dependency-version conflict rejection | Complete    |    18 |

---

# 28. Default Registry

| Feature                                 | Status      | Phase |
| --------------------------------------- | ----------- | ----: |
| `duongonix/vpm` registry                | Complete    |    18 |
| `vpm add math`                          | Complete    |    18 |
| `vpm add math@latest`                   | Complete    |    18 |
| `vpm add math@x.y.z`                    | Complete    |    18 |
| Stable SemVer sorting                   | Complete    |    18 |
| Malformed version filtering             | Complete    |    18 |
| Prerelease exclusion from stable latest | Complete    |    18 |

---

# 29. Self-Hosted Packages

| Feature                   | Status      | Phase |
| ------------------------- | ----------- | ----: |
| `owner/repo/package`      | Complete    |    18 |
| Deep package path         | Complete    |    18 |
| GitHub default provider   | Complete    |    18 |
| `gitlab:` provider prefix | Complete    |    18 |
| Repo-root package         | Removed     |     — |

---

# 30. Optimization

Status reflects the M1.1 audit: `vut-mir::optimize` implements constant
folding, dead-branch elimination, and per-block dead-code elimination. Every
other pass below is not yet implemented as a pass (release relies on
Cranelift's machine-level optimizer).

| Feature                    | Status      | Phase |
| -------------------------- | ----------- | ----: |
| Constant folding           | Complete    |    19 |
| Constant propagation       | Not Started |    19 |
| Dead-code elimination      | Complete    |    19 |
| Dead-branch elimination    | Complete    |    19 |
| Copy elision               | Not Started |    19 |
| Drop elimination           | Not Started |    19 |
| Last-use move optimization | Partial     |    19 |
| Escape analysis            | Not Started |    19 |
| Stack promotion            | Not Started |    19 |
| Scalar replacement         | Not Started |    19 |
| Allocation elimination     | Not Started |    19 |
| Allocation sinking         | Not Started |    19 |
| Bounds-check elimination   | Not Started |    19 |
| Interface devirtualization | Not Started |    19 |
| Retain/release elimination | Not Started |    19 |
| Basic inlining             | Not Started |    19 |
| CSE                        | Not Started |    19 |
| Expression simplification  | Not Started |    19 |

---

# 31. Incremental Compilation

Status reflects the M1.1 audit: the compiler uses a whole-program BLAKE3
fingerprint plus an object/executable artifact cache. There are no staged
parse/HIR/type-check/MIR caches and the dependency graph is not wired into
invalidation.

| Feature                        | Status      | Phase |
| ------------------------------ | ----------- | ----: |
| Source hashing                 | Complete    |    19 |
| BLAKE3 fingerprints            | Complete    |    19 |
| Module dependency invalidation | Not Started |    19 |
| Reverse dependency graph       | Not Started |    19 |
| Cache keys                     | Complete    |    19 |
| Cache schema version           | Complete    |    19 |
| Parse cache                    | Not Started |    19 |
| HIR cache                      | Not Started |    19 |
| Type-check cache               | Not Started |    19 |
| MIR cache                      | Not Started |    19 |
| Object cache                   | Complete    |    19 |
| Dependency build cache         | Not Started |    19 |
| Target-specific cache          | Partial     |    19 |
| Debug/release cache separation | Complete    |    19 |
| Corruption recovery            | Complete    |    19 |
| No-change minimal rebuild      | Complete    |    19 |

---

# 32. Tooling

| Feature                     | Status      | Phase |
| --------------------------- | ----------- | ----: |
| `vpm test`                  | Complete    |    20 |
| `vpm fmt`                   | Complete    |    20 |
| `vpm lint`                  | Complete    |    20 |
| `vpm doc`                   | Complete    |    20 |
| `vpm clean`                 | Complete    |    20 |
| `vpm tree`                  | Complete    |    20 |
| `vpm outdated`              | Complete    |    20 |
| `vpm search`                | Complete    |    20 |
| `vpm info`                  | Complete    |    20 |
| Canonical 2-space formatter | Complete    |    20 |
| Lossless syntax tree        | Complete    |    20 |
| Trivia-preserving formatter | Complete    |    20 |

---

# 33. Testing

| Feature                | Status      | Phase |
| ---------------------- | ----------- | ----: |
| Unit tests             | Complete    |   All |
| Integration tests      | Complete    |   All |
| Compile-pass tests     | Complete    |    20 |
| Compile-fail tests     | Partial     |    20 |
| Diagnostic snapshots   | Not Started | 04/20 |
| Lexer fuzzing          | Complete    | 02/20 |
| Parser fuzzing         | Complete    | 03/20 |
| Scheduled corpus fuzzing | Complete  |    20 |
| Test filtering         | Complete    |    20 |
| Test output capture    | Complete    |    20 |
| Assertion result API   | Complete    | 15/20 |
| Resolver tests         | Complete    |    05 |
| Type-system tests      | Complete    |    07 |
| Interface tests        | Complete    |    11 |
| Ownership tests        | Complete    |    12 |
| Memory-safety tests    | Complete    | 12/20 |
| AddressSanitizer CI    | Complete    |    20 |
| LeakSanitizer CI       | Complete    |    20 |
| Runtime ABI tests      | Complete    | 14/20 |
| Cross-platform CI      | Complete    |    20 |
| Native codegen tests   | Complete    |    13 |
| Runtime tests          | Complete    |    14 |
| Standard-library tests | Partial     |    15 |
| CLI tests              | Complete    |    16 |
| VPM tests              | Complete    | 17/18 |
| Package-provider tests | Complete    |    18 |
| Incremental tests      | Complete    |    19 |
| Benchmark suite        | Partial     | 19/20 |

---

# 34. Performance Benchmarks

| Benchmark                 | Status      | Phase |
| ------------------------- | ----------- | ----: |
| Lexer throughput          | Complete    |    02 |
| Parser throughput         | Complete    |    03 |
| Type-check throughput     | Complete    |    07 |
| Primitive operations      | Complete    |    19 |
| Small-data copy           | Not Started |    19 |
| Large-data move           | Not Started |    19 |
| Function argument passing | Complete    |    19 |
| Function return           | Complete    |    19 |
| List creation             | Not Started |    19 |
| List iteration            | Not Started |    19 |
| List mutation             | Not Started |    19 |
| Map access                | Complete    |    19 |
| String creation           | Complete    |    19 |
| String concatenation      | Complete    |    19 |
| Template strings          | Not Started |    19 |
| Interface dispatch        | Complete    |    19 |
| Devirtualized interface   | Not Started |    19 |
| Dyn values                | Complete    |    19 |
| Allocation-heavy workload | Not Started |    19 |
| Peak-memory tracking      | Not Started |    19 |
| Incremental warm build    | Complete    |    19 |
| No-change build           | Complete    |    19 |

---

# 35. Deferred Language Features

| Feature                          | Status   |
| -------------------------------- | -------- |
| Async/await (single-thread)      | Complete |
| Full concurrency                 | Deferred |
| Thread-sharing model             | Deferred |
| Full generic declaration syntax  | Complete |
| Generic constraints              | Complete |
| Payload-enum final syntax        | Complete |
| Static methods                   | Complete |
| Extension methods (same module)  | Complete |
| General overloading              | Deferred |
| Non-capturing lambdas            | Complete |
| Capturing closures               | Deferred |
| Reflection                       | Deferred |
| Explicit reference syntax        | Deferred |
| Source-level borrow checker      | Deferred |
| General user-defined destructors | Deferred |
| `dyn` value boxing/ownership/transport/display | Complete |
| `dyn` runtime type test / downcast / dispatch  | Deferred (no normative syntax) |
| Cross-module extension methods   | Deferred |
| FFI v1: `extern "C"`, `ptr(T)`, `@repr(C)`, `@link_name`, `unsafe:` | Complete |
| FFI callbacks (non-capturing Vut functions/lambdas) | Complete |
| Native static-library linking (`[native]`, CLI, LinkPlan) | Complete |
| `opaque data` native handles       | Complete |
| Raw pointer dereference/arithmetic | Deferred |
| By-value `repr(C)` struct ABI      | Deferred |
| Variadic parameters (`...args`)    | Partial  |
| Variadic view escape rejection     | Blocked (spec `04` §47a r3 forbids escape; `specs/22` defines no diagnostic code) |
| FFI varargs                        | Deferred |
| FFI dynamic libraries              | Deferred |
| Foreign-thread callbacks           | Deferred |
| Advanced unsafe API                | Deferred |

---

# 36. Deferred Toolchain Features

| Feature            | Status   |
| ------------------ | -------- |
| REPL               | Deferred |
| JIT                | Deferred |
| VM backend         | Deferred |
| LSP                | In Progress |
| Debugger           | Deferred |
| Package publishing | Deferred |
| VPM authentication | Deferred |
| Package yanking    | Deferred |

---

# 37. Implemented Post-MVP Subsystems

These are implemented and tested beyond the original MVP scope but were missing
from earlier tracker revisions (M1.1 audit).

| Subsystem                              | Status   | Evidence |
| -------------------------------------- | -------- | -------- |
| Generics (monomorphization, nested)    | Complete | `generics_e2e`, `generic_bounds_e2e` |
| Single-thread async / `Vutcon`         | Complete | `async*`, `vutcon*` e2e suites |
| Distribution/installer (`vut-dist`)    | Complete | `vut-dist` unit tests, `install/`, `specs/deploy/` |
| Receiver functions (`specs/receiver/`) | Complete | `receiver_e2e` |
| `dyn` boxing/ownership/display         | Complete | `dyn_e2e` (transport/store; dispatch is a separate gap) |

---

# 38. Removed Language Features

| Feature                                   | Status  |
| ----------------------------------------- | ------- |
| Anonymous record `$()`                    | Removed |
| Tuple type                                | Removed |
| Tuple literal                             | Removed |
| `while`                                   | Removed |
| Class                                     | Removed |
| Inheritance                               | Removed |
| `impl`                                    | Removed |
| `implements`                              | Removed |
| `pub`                                     | Removed |
| `private` keyword                         | Removed |
| `export`                                  | Removed |
| `is` operator                             | Removed |
| Global tracing GC                         | Removed |
| Global ARC memory model                   | Removed |
| `.vutlib` binary package format           | Removed |
| Repo-root VPM package                     | Removed |
| `println()` as core/basic output function | Removed |

---

# 38. Current MVP Success Criteria

The MVP is considered complete when all required features above are `Complete` and this workflow succeeds:

```bash
vpm new hello
cd hello
vpm run
```

with a real native executable.

A representative Vut program must be able to use:

```vut
data User:
  name: str
  age: int

fn User.greet():
  out("Hello $self.name")

fn main():
  name = input("Your name: ")

  user = User(
    name = name,
    age = 20
  )

  print("Welcome ")
  out("$user.name")

  values = @(10, 20, 30)

  for value, index in values:
    out("$index -> $value")
```

and compile through the complete native pipeline.

---

# 39. Update Rule

After completing a feature:

1. update its status in this file
2. update the corresponding phase file
3. record important implementation decisions
4. record known limitations
5. add or update tests
6. add benchmarks for performance-sensitive features

Do not mark a feature `Complete` when it is only parsed, stubbed or partially implemented.

`Complete` means the feature works through every required compiler/runtime layer.
