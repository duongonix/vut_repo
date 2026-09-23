
# Vut High-Level Intermediate Representation

Template strings lower to text/expression segments. Each interpolation keeps
its source span and resolved static type for formatter/codegen selection; no
segment is implicitly converted to `dyn`.

## 1. Purpose

HIR is Vut's resolved high-level semantic representation.

Conceptually:

```text
AST
 ↓
resolution / semantic lowering
 ↓
HIR
 ↓
type checking / interface checking
 ↓
lowering
```

HIR removes syntax-level ambiguity and replaces textual references with compiler IDs where possible.

---

## 2. Goals

HIR should be:

```text
compact
resolved
typed-friendly
diagnostic-aware
stable enough for semantic passes
easy to traverse
incremental-friendly
```

---

## 3. AST vs HIR

AST:

```text
source-oriented
names as written
syntactic structure
```

HIR:

```text
semantic-oriented
resolved symbols
normalized constructs
compiler IDs
```

Example AST:

```text
Identifier("user")
```

HIR after resolution:

```text
SymbolRef(SymbolId(42))
```

---

## 4. HIR IDs

Use compact IDs where appropriate:

```text
HirId
SymbolId
ModuleId
TypeId
FunctionId
DataId
InterfaceId
EnumId
```

Do not pass raw indexes interchangeably.

Use newtypes.

---

## 5. Arenas

HIR is a strong candidate for indexed arena storage.

Conceptually:

```text
HirArena
HirExprId
HirStmtId
```

Potential implementation may use:

```text
la-arena
custom typed arena
```

after architecture review.

---

## 6. Source Origin

HIR nodes should retain source origin.

Conceptually:

```text
HirNode
├── semantic data
└── Span
```

Later diagnostics must still point to user source.

---

## 7. Module HIR

Conceptually:

```text
HirModule
├── ModuleId
├── declarations
├── imports
└── source
```

Module imports should already be resolved or associated with resolver results before dependent semantic passes.

---

## 8. Symbols

HIR should refer to declarations through:

```text
SymbolId
```

instead of repeating names.

Name strings remain available through symbol tables for diagnostics.

---

## 9. Functions

HIR function:

```text
HirFunction
├── FunctionId
├── symbol
├── is_async: bool
├── parameters
├── return_type
├── body
└── span
```

`return_type` is the logical result type. Async call results use the
compiler-internal awaitable type. Return type may initially contain inference
variables before final type checking.

---

## 10. Methods

Methods should be normalized into explicit receiver semantics internally.

Source:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

HIR may conceptually treat it as:

```text
method receiver = Counter
implicit self symbol created
parameters = amount
```

The compiler-injected `self` becomes an explicit semantic symbol internally.

---

## 11. Data

HIR data declaration should contain resolved field type references.

Conceptually:

```text
HirData
├── DataId
├── fields
└── methods/reference links
```

---

## 12. Interface

HIR interface requirements use resolved type IDs/references.

Structural matching should not repeatedly parse textual signatures.

---

## 13. Imports

AST import syntax such as:

```vut
import ..shared at foo
```

should lower to resolved module/symbol references.

HIR does not need to retain complex unresolved relative-dot semantics once resolution succeeds.

Keep source representation only as diagnostic metadata where useful.

---

## 14. Types

HIR should use semantic type representation.

Examples:

```text
TypeId(int)
TypeId(list[int])
TypeId(User)
```

Do not repeatedly resolve source `AstType` during later passes.

---

## 15. Expressions

Conceptual HIR expression kinds:

```text
Literal
Local
Global
Binary
Unary
Call
MethodCall
FieldAccess
Construct
List
Record
If
Match
Range
Await
```

`Await` holds the awaited expression and resolves to the logical result type of
an internal awaitable operand. Async calls and awaits are recorded in semantic
side tables rather than as strings.

Parser-level grouping nodes may disappear because parentheses have no runtime semantics.

---

## 16. Calls

HIR should distinguish semantic call forms once resolution knows the target:

```text
FunctionCall
MethodCall
ConstructorCall
InterfaceCall
```

This simplifies type checking/lowering.

---

## 17. Member Resolution

AST:

```text
user.name
```

HIR should eventually know whether this is:

```text
field
method
module symbol
```

after resolution/type information permits it.

When type information is needed to disambiguate, resolution and type checking may cooperate through clearly defined passes.

Do not use ad-hoc repeated string resolution.

---

## 18. Control Flow Normalization

HIR may normalize surface variants.

Example:

```vut
for value, index in items:
```

may remain a high-level `ForIterable` HIR node.

Lower-level iteration mechanics belong to MIR/lowering.

Do not lower directly to machine loops too early.

---

## 19. `if` Expressions

HIR should represent whether `if` is used as a value through context/type analysis rather than maintain separate unrelated implementations.

---

## 20. Match

HIR patterns should use resolved enum variants where known.

Example:

```text
Status.pending
```

should map to semantic enum/variant IDs.

---

## 21. Error HIR

When AST contains recovery/error nodes, HIR may contain:

```text
HirError
```

or skip generation while recording invalid IDs.

Later passes must avoid cascading diagnostics.

---

## 22. HIR Lowering

AST-to-HIR lowering should:

```text
resolve declarations
establish scopes
assign IDs
normalize methods/self
resolve type names where possible
normalize imports
preserve spans
```

Exact ordering may involve module/name resolver passes.

---

## 23. HIR Mutability

Prefer stable immutable HIR after construction where practical.

Semantic tables may store inferred types separately:

```text
HirExprId -> TypeId
```

instead of mutating every HIR node repeatedly.

This helps incremental compilation.

---

## 24. Side Tables

Useful semantic side tables may include:

```text
expr_types
symbol_refs
resolved_calls
field_refs
control_flow facts
```

Do not make HIR nodes enormous by storing every analysis result inline.

---

## 25. Performance

HIR should reduce repeated work.

Avoid:

```text
repeated name lookup
repeated type parsing
repeated module path normalization
large cloned strings
```

Prefer compact IDs.

---

## 26. Diagnostics

Every resolved semantic object must still be able to produce source diagnostics through stored spans/source references.

---

## 27. Tests

Test:

```text
AST → HIR lowering
resolved SymbolIds
implicit self insertion
type-name resolution
import resolution integration
call-kind normalization
error-node behavior
span preservation
```

---

## 28. Rules

1. HIR is semantic, AST is syntactic.
2. Replace textual names with IDs after resolution.
3. Preserve source spans.
4. Normalize implicit `self`.
5. Use compact typed IDs.
6. Avoid repeated semantic lookup.
7. HIR remains backend-independent.
8. Keep machine-specific representation out of HIR.
9. Use side tables where appropriate.
10. Design HIR for incremental reuse.
11. Async functions carry an `is_async` flag and `await` is a dedicated node.
