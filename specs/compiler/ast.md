
# Vut Abstract Syntax Tree

## 1. Purpose

AST represents Vut source after parsing.

AST is primarily syntax-oriented.

Conceptually:

```text
source
↓
tokens
↓
AST
↓
semantic lowering
↓
HIR
```

AST must preserve enough source structure for:

```text
diagnostics
semantic analysis
formatter/tooling support where appropriate
```

---

## 2. AST Responsibilities

AST represents:

```text
declarations
expressions
statements
types as written
identifiers
imports
blocks
source spans
```

AST should not contain fully resolved semantic information such as:

```text
SymbolId
final TypeId
resolved ModuleId
vtable layout
machine representation
```

String expressions retain typed-to-be-checked segments: `Text(span)` and
`Expr(AstExpr)`. The AST contains neither a runtime template program nor an
anonymous-record node.

Those belong to HIR or later stages.

---

## 3. Node IDs

Important AST nodes may receive:

```text
AstNodeId
```

if stable identity is useful.

Do not require IDs on every trivial node unless architecture benefits.

---

## 4. Source Spans

Every meaningful AST node should preserve:

```text
Span
```

Examples:

```text
declaration span
identifier span
operator span
expression span
type annotation span
```

Precise child spans are important for diagnostics.

---

## 5. Source File

Conceptually:

```text
AstFile
├── SourceId
└── items: list[AstItem]
```

---

## 6. Top-Level Item

Conceptual enum:

```text
AstItem
├── Import
├── Function
├── Method
├── Data
├── Interface
├── Enum
├── TypeAlias
└── Binding
```

Use typed Rust enums rather than string tags.

---

## 7. Functions

Conceptually:

```text
AstFunction
├── name
├── parameters
├── return_type?
├── is_async: bool
├── body
└── span
```

`is_async` is a typed flag, not a string. An `async fn` is an ordinary function
declaration with this flag set.

---

## 8. Methods

Source methods use the same required `fn` prefix as free functions:

```vut
fn Counter.increment():
  self.value = self.value + 1
```

Conceptually:

```text
AstMethod
├── receiver_type_name
├── method_name
├── parameters
├── return_type?
├── body
└── span
```

No explicit `self` field is needed in source parameters.

---

## 9. Data

Conceptually:

```text
AstData
├── name
├── fields
└── span
```

Field:

```text
AstDataField
├── name
├── type_expr
├── default_value?
└── span
```

---

## 10. Interface

Conceptually:

```text
AstInterface
├── name
├── parents
├── methods
└── span
```

Interface method requirement:

```text
name
parameters
return_type?
span
```

---

## 11. Enum

```text
AstEnum
├── name
├── variants: AstEnumVariant
└── span

AstEnumVariant
├── name
├── fields: AstVariantField
└── span

AstVariantField
├── name
├── type_expr
└── span
```

A variant with no fields is payloadless. Field syntax matches `data` fields.

---

## 12. Import

AST should preserve structured path information.

Conceptually:

```text
AstImport
├── path
├── mode
└── span
```

Mode:

```text
WholeModule
Alias(name)
Selected(names)
```

Relative path should preserve parent-depth semantics explicitly.

Do not keep it only as one opaque source string.

---

## 13. Type Expressions

AST type syntax should represent source-level type forms.

Conceptually:

```text
AstType
├── Named
├── Applied
└── Optional
```

Examples:

```text
int
User
list[int]
map[str, int]
User?
```

---

## 14. Statements

Conceptual:

```text
AstStmt
├── Binding
├── Assignment
├── Expression
├── If
├── For
├── Break
├── Continue
├── Return
└── Error
```

Exact split between statement/expression nodes may follow Vut's expression-oriented design.

---

## 15. Expressions

Conceptual:

```text
AstExpr
├── Identifier
├── Integer
├── Float
├── String
├── Bool
├── Null
├── List
├── Array
├── TemplateString
├── Binary
├── Unary
├── Member
├── Call
├── Await
├── If
├── Match
├── Range
├── Group
└── Error
```

`Await` is a dedicated node holding the awaited operand and its span. It is not
represented as a call or a string flag.

Methods carry the same `is_async` flag as free functions.

`List` and `Array` are distinct syntax nodes. The parser emits `List` only for
`@[...]` and `Array` only for `[...]`; neither is represented as a generic
call or reinterpreted using its expected type.

---

## 16. Literals

Numeric literals should preserve source representation or parsed literal metadata without prematurely committing to final type.

Example:

```text
123
```

does not need to become `i64` immediately.

Type inference decides final type.

---

## 17. Binary Operators

Use an enum:

```text
Add
Subtract
Multiply
Divide
Modulo
Equal
NotEqual
Less
LessEqual
Greater
GreaterEqual
And
Or
```

Do not store operator semantic identity only as source text.

---

## 18. Unary Operators

Use enum values such as:

```text
Not
Negate
Positive
```

---

## 19. For Representation

Prefer one unified node:

```text
AstFor
├── kind
├── body
└── span
```

Possible kind enum:

```text
Infinite
Conditional(expr)
Iterable {
  value_binding,
  index_binding?,
  iterable
}
```

This simplifies later semantic lowering.

---

## 20. Match

Basic representation:

```text
AstMatch
├── value
├── arms
└── span
```

Arm:

```text
pattern
guard: expression?
expression
span
```

Patterns are a recursive typed enum:

```text
AstMatchPattern
├── Wildcard
├── Binding(name)
├── Literal(value)
├── Variant { name, fields }
├── ResultOk { pattern }
├── ResultErr { pattern }
├── Or { alternatives }
├── Range { start, end, inclusive }
├── List { items, rest? }
├── Group { pattern }
└── Error
```

A bare identifier is a `Binding`; the type checker reinterprets it as a variant
pattern when the scrutinee enum has a variant of that name.

---

## 21. Bindings

Distinguish source forms as needed:

```text
name = expr
name: Type = expr
```

AST should preserve optional explicit type annotation.

Constant/private status may be derived later from identifier spelling.

---

## 22. Recovery Nodes

AST must tolerate parser recovery.

Use explicit error variants rather than fake valid syntax.

Later phases should skip/suppress dependent diagnostics.

---

## 23. AST Ownership

Avoid excessive deep clones.

Use:

```text
arena allocation
indexed nodes
compact vectors
owned tree structures
```

according to measured architecture.

Do not clone AST between every compiler phase.

---

## 24. AST vs Lossless Syntax Tree

AST does not necessarily need to preserve every whitespace/comment token.

If formatter/LSP requires a lossless tree, maintain a separate syntax representation.

Do not pollute semantic AST solely for formatting trivia.

---

## 25. AST Stability

AST is an internal compiler representation.

It is not a stable public language API.

It may evolve with the compiler.

---

## 26. Tests

Test:

```text
correct node kind
correct spans
nested expressions
declarations
methods
data
interfaces
imports
for variants
match
recovery nodes
```

---

## 27. Rules

1. AST remains syntax-oriented.
2. Preserve precise source spans.
3. Use typed enums.
4. Do not store final semantic types in AST.
5. Do not resolve modules in AST.
6. Avoid giant universal nodes with string tags.
7. Recovery nodes are explicit.
8. AST is internal and evolvable.
9. Avoid unnecessary deep clones.
10. HIR, not AST, becomes the primary resolved semantic representation.
11. Async declarations use an `is_async` flag and `await` uses a dedicated node.
