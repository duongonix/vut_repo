
# Vut Parser

## 1. Purpose

The parser transforms lexer tokens into Vut syntax structures.

Conceptually:

```text
TokenStream
    ↓
Parser
    ↓
AST
```

The parser validates syntax, not full semantics.

---

## 2. Responsibilities

Parser handles:

```text
declarations
statements
expressions
operator precedence
blocks
imports
data syntax
interfaces
enums
functions
methods
control flow
error recovery
```

Parser does not perform:

```text
type inference
interface satisfaction
package resolution
module filesystem resolution
code generation
```

---

## 3. Architecture

Parser must be modular.

Recommended:

```text
parser/
├── mod.rs
├── declaration.rs
├── expression.rs
├── statement.rs
├── function.rs
├── data.rs
├── interface.rs
├── enum.rs
├── import.rs
├── control_flow.rs
└── recovery.rs
```

Do not place the complete parser in one giant file.

---

## 4. Parsing Strategy

Recommended direction:

```text
recursive descent
+
Pratt parser for expressions
```

This gives precise control over:

```text
Vut syntax
error recovery
source spans
diagnostic quality
```

A parser library may be used if it materially improves these properties.

Do not compromise diagnostics merely to use a framework.

---

## 5. Input

Parser consumes:

```text
Token
TokenKind
Span
```

Template-string interpolation is parsed into text and expression segments. The
embedded expression uses the ordinary expression parser; recovery preserves
precise spans for invalid or unclosed interpolation.

It should never need to re-lex source fragments.

---

## 6. Output

Parser produces:

```text
AST
+
syntax diagnostics
```

AST nodes should preserve source ranges.

Invalid syntax may create recoverable error nodes so later declarations can still be parsed.

---

## 7. Source File

Parse:

```text
source_file
```

as a sequence of top-level items until EOF.

Supported top-level constructs follow:

```text
specs/21-grammar.md
```

---

## 8. Functions

Parse:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Capture:

```text
name
parameters
return type
body
span
```

---

## 9. Methods

Parse:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

Methods require:

```text
fn
```

and must not contain an explicit `self` parameter. A top-level declaration
matching `Type.method(...):` without `fn` is rejected with an actionable
declaration diagnostic; it is not accepted through a compatibility path.

Explicit-self rejection may be semantic or parser-level depending on AST design.

---

## 10. Data

Parse:

```vut
data User:
  name: str
  age: int
  active: bool = true
```

Field declarations inside data blocks have specialized grammar.

Do not treat them as arbitrary statements.

---

## 11. Interfaces

Parse:

```vut
interface Animal:
  speak() -> str
```

and:

```vut
interface ReadWriter: Reader, Writer:
  close()
```

The parser must disambiguate parent-interface list from block-opening colon cleanly.

---

## 12. Enums

Parse basic enums:

```vut
enum Status:
  pending
  running
  done
```

Do not invent payload enum syntax.

---

## 13. Imports

Parse exactly:

```vut
import math
import math as m
import math at add, plus
```

Reject combined:

```vut
import math as m at add
```

with a precise diagnostic.

---

## 14. Relative Imports

Preserve leading-dot count.

Examples:

```vut
import .c
import ..utils
import ...math
```

Parser should not perform filesystem traversal.

It only produces a structured path.

---

## 15. `if`

Parse both statement/expression form.

Example:

```vut
if active:
  run()
else:
  stop()
```

Expression:

```vut
value = if active:
  10
else:
  20
```

Prefer one AST representation if practical.

---

## 16. `for`

Support exactly:

```vut
for:
  ...

for condition:
  ...

for item in items:
  ...

for value, index in items:
  ...
```

No `while`.

No C-style `for`.

No three-variable iterable binding.

---

## 17. `match`

Parse:

```vut
text = match status:
  pending: "Waiting"
  running: "Running"
  done: "Done"
```

Current patterns remain intentionally limited.

---

## 18. Expressions

Use defined precedence.

Recommended order:

```text
or
and
comparisons
ranges
addition/subtraction
multiplication/division/modulo
unary
postfix
primary
```

Do not rely on parser implementation accident for precedence.

---

## 19. Pratt Parser

Expression parsing should assign explicit binding powers to operators.

This allows easy extension while maintaining predictable precedence.

Operator tables should be centralized rather than scattered through parser code.

---

## 20. Postfix Parsing

Postfix parsing handles chains such as:

```vut
user.profile.name
math.add(1, 2)
user.name.to_str()
```

Conceptually:

```text
primary
↓
member/call
↓
member/call
...
```

---

## 21. Constructor vs Function Call

Parser should not determine whether:

```vut
User(name = "Ha")
```

is a constructor or function call.

Represent it as call-like syntax.

Semantic resolution determines callee kind.

---

## 22. Parentheses

Parentheses group expressions:

```vut
(a + b) * c
```

They do not create tuples.

Reject:

```vut
(a, b)
```

when no defined construct permits it.

---

## 23. Named Arguments

Parse:

```vut
User(
  name = "Ha",
  age = 20
)
```

Named-argument validity is semantic.

Parser records:

```text
name
value
span
```

---

## 24. Block Parsing

A block begins after:

```text
:
NEWLINE
INDENT
```

and ends on:

```text
DEDENT
```

where lexer token model uses those tokens.

---

## 25. Missing Colon

Special-case common errors.

Example:

```vut
if active
  run()
```

should produce:

```text
E0102: expected `:`
```

rather than a generic unexpected-token cascade.

---

## 26. Recovery

Recovery is mandatory.

Useful synchronization points:

```text
NEWLINE
DEDENT
EOF
fn
data
interface
enum
import
type
```

Parser should resume at meaningful boundaries.

---

## 27. Error Nodes

AST may include:

```text
ErrorExpr
ErrorStmt
ErrorDecl
```

or equivalent recovery representations.

Later semantic phases must recognize and suppress cascades from these nodes.

---

## 28. No Semantic Guessing

Parser must not reject code because:

```text
types differ
symbol is unknown
interface is missing method
module path does not exist
```

Those belong to later phases.

---

## 29. Diagnostics

Each syntax diagnostic should provide:

```text
error code
primary span
message
expected token where meaningful
help where reliable
```

Follow:

```text
09-errors-diagnostics.md
22-error-codes.md
```

---

## 30. Tests

Parser tests must cover:

```text
every declaration
every statement
every expression precedence level
valid nesting
invalid nesting
missing colon
missing parenthesis
unexpected indentation
invalid import
invalid method declaration
tuple rejection
no while
recovery across declarations
```

---

## 31. Snapshot Tests

AST snapshots may be useful for parser regression tests.

Do not expose debug AST output as public language API.

---

## 32. Performance

Avoid:

```text
token cloning
backtracking over large regions
repeated parsing of same tokens
allocating temporary strings
```

Parser should operate approximately linearly for normal deterministic grammar.

---

## 33. Rules

1. Parser validates syntax, not semantics.
2. Parser is modular.
3. Expression precedence is explicit.
4. Error recovery is mandatory.
5. AST preserves source spans.
6. Calls are syntactic; semantic resolution determines meaning.
7. Tuple syntax is rejected.
8. No `while`.
9. No unsupported speculative syntax.
10. Parser must never panic on invalid user input.
