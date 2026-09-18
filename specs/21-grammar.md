# Vut Formal Grammar

## 1. Purpose

This document defines the formal grammar direction of the Vut programming language.

It is intended to guide:

* lexer implementation
* parser implementation
* formatter implementation
* syntax diagnostics
* language-server parsing
* future grammar evolution

This grammar describes Vut's surface syntax.

Semantic rules such as type compatibility, visibility, interface satisfaction and package resolution are defined in their dedicated specifications.

---

## 2. Grammar Notation

This document uses an EBNF-like notation.

Conventions:

```text
"token"      exact source token
name         grammar rule
[ item ]     optional
{ item }     zero or more
item | item  alternatives
```

Indentation-sensitive block handling is represented conceptually and may be lowered internally into:

```text
NEWLINE
INDENT
DEDENT
```

tokens.

---

## 3. Source File

```text
source_file :=
    { top_level_item }
    EOF
```

---

## 4. Top-Level Items

```text
top_level_item :=
    import_statement
  | function_declaration
  | method_declaration
  | data_declaration
  | interface_declaration
  | enum_declaration
  | type_alias
  | assignment_statement
```

Additional top-level constructs may be added only after their syntax is formally specified.

---

## 5. Newlines

Vut uses newlines rather than semicolons to terminate normal statements.

Conceptually:

```text
statement NEWLINE
```

Blank lines may appear between statements.

Semicolons are not part of canonical Vut syntax.

---

## 6. Indentation Blocks

A block begins after `:` and a newline.

Conceptually:

```text
block :=
    ":" NEWLINE INDENT { statement } DEDENT
```

A block may also be used as an expression, where its value is the final
expression of the block (or `void` when the final statement is not an
expression):

```text
block_expression :=
    NEWLINE INDENT { statement } DEDENT
```

Block expressions are accepted in `match` arm bodies and as the right-hand side
of an assignment (`name = NEWLINE INDENT ... DEDENT`). They are not accepted
inside parentheses, template interpolation, or loop/condition headers.

Example:

```vut
if active:
  print("active")
```

The lexer/parser implementation may represent indentation differently internally, but observable grammar must remain equivalent.

---

## 7. Identifiers

Conceptually:

```text
identifier :=
    identifier_start { identifier_continue }

identifier_start :=
    letter
  | "_"

identifier_continue :=
    letter
  | digit
  | "_"
```

An identifier cannot begin with a digit.

Identifiers are case-sensitive.

---

## 8. Keywords

Reserved/core keywords currently include:

```text
fn
data
interface
enum
type

async
await

if
elif
else
match

for
in
break
continue

return

import
as
at

and
or
not

true
false
null

dyn
```

Potential future keywords such as:

```text
spawn
thread
channel
select
```

must only become lexer-reserved once their grammar is finalized and implemented.

Do not reserve large sets of speculative keywords unnecessarily.

---

## 9. Literals

```text
literal :=
    integer_literal
  | float_literal
  | string_literal
  | boolean_literal
  | null_literal
  | list_literal
```

---

## 10. Boolean Literal

```text
boolean_literal :=
    "true"
  | "false"
```

---

## 11. Null Literal

```text
null_literal :=
    "null"
```

---

## 12. String Literal

Canonical string literals use double quotes.

Conceptually:

```text
string_literal :=
    '"' { string_character | escape_sequence } '"'
```

Exact escaping rules belong to the lexer specification.

---

## 13. Integer Literal

Conceptually:

```text
integer_literal :=
    digit { digit }
```

Additional numeric formats such as:

```text
hexadecimal
binary
octal
numeric separators
```

must be explicitly specified before parser support is added.

---

## 14. Floating-Point Literal

Conceptually:

```text
float_literal :=
    digit { digit } "." digit { digit }
```

Exponent forms and special numeric syntax require explicit specification before implementation.

---

## 15. List Literal

```text
list_literal :=
    "@(" [ expression { "," expression } [ "," ] ] ")"

array_literal :=
    "array(" expression { "," expression } [ "," ] ")"

array_type :=
    "array(" type_expression "," integer_literal ")"
```

Examples:

```vut
@()
@(1, 2, 3)
@(
  1,
  2,
  3
)
```

The parser accepts layout/newlines according to delimiter rules.

Type homogeneity is a semantic rule, not a parser rule.
An array literal must contain at least one element. `@(...)` always denotes a
list literal and `array(...)` always denotes an array literal; contextual
reinterpretation is forbidden.

---

## 16. Type Syntax

```text
type_expression :=
    named_type
  | parameterized_type
  | optional_type
  | function_type

function_type :=
    "fn" "(" [ type_expression { "," type_expression } ] ")"
    [ "->" type_expression ]

receiver_function_type :=
    "fn" "(" type_expression ")" "(" [ type_expression { "," type_expression } ] ")"
    [ "->" type_expression ]
```

`receiver_function_type` declares a distinct receiver, e.g.
`fn(ColumnScope)() -> void`; it is not the ordinary single-parameter type
`fn(ColumnScope)`, nor the curried type `fn(ColumnScope) -> fn(...) -> ...`.
See `specs/receiver/`.

---

## 17. Template Strings

```text
string_literal := '"' { string_text | escape | interpolation } '"'
escape := "\\" ( "\\" | '"' | "n" | "r" | "t" | "$" )
interpolation := "$" identifier | "$(" expression ")"
```

The expression uses ordinary expression grammar and matching its final `)`
handles nested parentheses. `$()` is never an expression outside a string.

---

## 18. Named Type

```text
named_type :=
    qualified_identifier
```

Examples:

```text
int
str
User
math.Vector
```

A qualified named type is allowed when its first segment names an imported
module (for example `import math` then `math.Vector`). Intermediate segments
follow module targets, and the final segment must name a public symbol of the
target module. Module-qualified type references and selected imports (`import
math at Vector`) are equivalent ways to refer to the same type.

---

## 19. Parameterized Type

```text
parameterized_type :=
    identifier "(" type_expression { "," type_expression } ")"
```

Examples:

```text
list(int)
map(str, int)
result(User, Error)
ptr(u8)
```

Vut does not use angle-bracket generic application.

A parameterized type name is unqualified; qualified generic application such as
`math.Page(int)` is not part of the grammar.

---

## 20. Optional Type

```text
optional_type :=
    primary_type "?"
```

Example:

```text
str?
User?
```

The parser should preserve optional syntax distinctly enough for type lowering.

---

## 21. Type Alias

```text
type_alias :=
    "type" identifier "=" type_expression NEWLINE
```

Example:

```vut
type UserId = u64
```

---

## 22. Assignment

```text
assignment_statement :=
    assignment_target [ ":" type_expression ] "=" expression NEWLINE
```

Examples:

```vut
age = 20
age: int = 20
```

---

## 23. Assignment Target

Initially:

```text
assignment_target :=
    identifier
  | member_expression
```

Examples:

```vut
age = 20
user.age = 21
```

More complex destructuring assignment is not currently defined.

---

## 24. Constants

Constants do not have separate grammar.

The parser treats:

```vut
MAX_SIZE = 100
```

as a normal binding.

Semantic analysis determines that ALL-CAPS identifiers are constants.

---

## 25. Function Declaration

```text
function_declaration :=
    [ "async" ] "fn" identifier
    "(" [ parameter_list ] ")"
    [ "->" type_expression ]
    block
```

Example:

```vut
fn add(a: int, b: int) -> int:
  a + b
```

Async:

```vut
async fn fetch() -> Data:
  data = await read_data()
  data
```

`async` is a declaration modifier and is only valid directly before `fn`.

---

## 26. Parameters

```text
parameter_list :=
    parameter { "," parameter }

parameter :=
    identifier ":" type_expression
  | "..." identifier ":" type_expression
```

A `...name: T` parameter is the trailing variadic parameter: it accepts zero or
more arguments of type `T` and must be last.

Example:

```vut
fn add(a: int, b: int):
  ...
```

Untyped function parameters are not part of the current grammar.

---

## 27. Method Declaration

```text
method_declaration :=
    [ "async" ] "fn" qualified_type_name "." identifier
    "(" [ parameter_list ] ")"
    [ "->" type_expression ]
    block
```

Example:

```vut
fn Counter.add(amount: int):
  self.value = self.value + amount
```

Async:

```vut
async fn Counter.refresh() -> bool:
  value = await fetch()
  value
```

`self` is not written in the parameter list.

The removed form `Type.method(...):` without the leading `fn` does not match
`method_declaration` and is a compile error.

---

## 28. Data Declaration

```text
data_declaration :=
    "data" identifier block_of_data_fields
```

Conceptually:

```text
block_of_data_fields :=
    ":" NEWLINE INDENT { data_field NEWLINE } DEDENT

data_field :=
    identifier ":" type_expression [ "=" expression ]
```

Example:

```vut
data User:
  name: str
  age: int
  active: bool = true
```

---

## 29. Data Construction

Construction uses ordinary call-like syntax:

```text
construction_expression :=
    qualified_identifier "(" [ named_argument_list ] ")"
```

Named arguments:

```text
named_argument_list :=
    named_argument { "," named_argument }

named_argument :=
    identifier "=" expression
```

Example:

```vut
User(
  name = "Ha",
  age = 20
)
```

Semantic analysis determines whether the callee is a type constructor or a function.

---

## 30. Interface Declaration

```text
interface_declaration :=
    "interface" identifier
    [ ":" interface_parent_list ]
    interface_block
```

Because `:` is also the block opener, parser grammar must distinguish:

```vut
interface Animal:
  speak() -> str
```

from:

```vut
interface ReadWriter: Reader, Writer:
  close()
```

A practical grammar may parse header tokens until the final block-start colon.

---

## 31. Interface Parent List

Conceptually:

```text
interface_parent_list :=
    identifier { "," identifier }
```

Example:

```vut
interface ReadWriter: Reader, Writer:
  close()
```

Exact parser formulation should avoid ambiguity with the block colon.

---

## 32. Interface Method Requirement

```text
interface_method :=
    identifier
    "(" [ parameter_list ] ")"
    [ "->" type_expression ]
    NEWLINE
```

Example:

```vut
interface Reader:
  read(size: int) -> bytes
```

No body exists.

No `self` parameter is declared.

---

## 33. Enum Declaration

Basic enum grammar:

```text
enum_declaration :=
    "enum" identifier enum_block

enum_block :=
    ":" NEWLINE INDENT
    enum_variant { NEWLINE enum_variant }
    DEDENT

enum_variant :=
    identifier [ "(" [ variant_field { "," variant_field } ] ")" ]

variant_field :=
    identifier ":" type_expression
```

Example:

```vut
enum Status:
  pending
  running
  done
```

Payload:

```vut
enum Shape:
  point
  circle(radius: float)
  rect(width: float, height: float)
```

Variant payload fields use `name: Type`, matching `data` field syntax.

---

## 34. Import Statement

```text
import_statement :=
    "import" import_path [ import_suffix ] NEWLINE
```

---

## 35. Import Suffix

```text
import_suffix :=
    "as" identifier
  | "at" identifier { "," identifier }
```

Allowed:

```vut
import math
import math as m
import math at add, plus
```

Do not allow combined `as` + `at` in Vut v1.

---

## 36. Absolute Import Path

Conceptually:

```text
absolute_import_path :=
    identifier { "." identifier }
```

Example:

```vut
import app.user.parser
```

---

## 37. Relative Import Path

Relative paths begin with one or more dots.

Semantics:

```text
.       current directory
..      up 1
...     up 2
....    up 3
```

Conceptually:

```text
relative_import_path :=
    relative_prefix identifier { "." identifier }
```

The lexer/parser must preserve the count of leading dots.

---

## 38. Import Path

```text
import_path :=
    absolute_import_path
  | relative_import_path
```

Filesystem resolution is semantic, not syntax. For each local module segment,
the resolver tries `name.vut` before `name/mod.vut`; if both exist, the file
candidate wins without ambiguity.

---

## 39. `if`

```text
if_statement :=
    "if" expression block
    { "elif" expression block }
    [ "else" block ]
```

Example:

```vut
if age >= 18:
  print("adult")
elif age >= 13:
  print("teen")
else:
  print("child")
```

---

## 40. `if` Expression

Vut permits an `if` construct in expression position.

The parser may represent statement/expression `if` using the same AST form where practical.

Example:

```vut
max = if a > b:
  a
else:
  b
```

Semantic analysis determines whether branches produce compatible values.

---

## 41. `match`

Basic grammar direction:

```text
match_expression :=
    "match" expression ":" NEWLINE INDENT
    match_arm { NEWLINE match_arm }
    DEDENT

match_arm :=
    match_pattern [ "if" expression ] ":" ( expression | block )
```

An arm body may be a single inline expression or an indented block. The block's
value is its final expression, so a nested `match` can appear as a multi-line arm
body.

Example:

```vut
text = match status:
  pending: "Waiting"
  running: "Running"
  done: "Done"
```

---

## 42. Match Pattern

```text
match_pattern :=
    or_pattern

or_pattern :=
    primary_pattern { "or" primary_pattern }

primary_pattern :=
    "_"
  | literal_pattern
  | range_pattern
  | "(" match_pattern ")"
  | "@" "(" [ list_pattern_items ] ")"
  | identifier
  | identifier "(" [ variant_field_patterns ] ")"
  | "ok" "(" match_pattern ")"
  | "err" "(" match_pattern ")"

literal_pattern :=
    integer_literal
  | float_literal
  | string_literal
  | "true" | "false"
  | "null"

range_pattern :=
    integer_literal ( ".." | "..=" ) integer_literal

variant_field_patterns :=
    variant_field_pattern { "," variant_field_pattern }

variant_field_pattern :=
    identifier "=" match_pattern
  | match_pattern

list_pattern_items :=
    [ match_pattern { "," match_pattern } [ "," rest_pattern ] ]
  | rest_pattern

rest_pattern :=
    ".."
  | identifier ".."
```

A bare identifier is a binding unless the scrutinee is an enum with a variant
of that name, in which case it is a variant pattern.

Examples:

```vut
match value:
  1 or 2: ...
  3..=9: ...
  "text": ...
  _: ...

match shape:
  circle(radius = r): ...
  rect(width = w, height = h): ...
```

List patterns are parsed but reserved; the type checker reports `E7112` until
their memory semantics are implemented.

---

## 43. Infinite `for`

```text
for_statement :=
    "for" ":" block_body
```

Conceptually equivalent surface form:

```vut
for:
  work()
```

---

## 44. Conditional `for`

```text
for_statement :=
    "for" expression block
```

Example:

```vut
for running:
  work()
```

Semantic checking requires the expression to be `bool`.

---

## 45. Iterable `for`

```text
for_statement :=
    "for" identifier "in" expression block
```

Example:

```vut
for item in items:
  print(item)
```

---

## 46. Indexed Iterable `for`

```text
for_statement :=
    "for" identifier "," identifier "in" expression block
```

Example:

```vut
for value, index in items:
  print(value)
  print(index)
```

The first binding is value.

The second is index.

---

## 47. Combined `for` Grammar

Conceptually:

```text
for_statement :=
    "for" ":" block_body
  | "for" expression ":" block_body
  | "for" identifier "in" expression ":" block_body
  | "for" identifier "," identifier "in" expression ":" block_body
```

The actual parser should resolve ambiguities using token lookahead rather than duplicating semantic logic.

---

## 48. `break`

```text
break_statement :=
    "break" NEWLINE
```

Labels are not currently defined.

---

## 49. `continue`

```text
continue_statement :=
    "continue" NEWLINE
```

Labels are not currently defined.

---

## 50. `return`

```text
return_statement :=
    "return" [ expression ] NEWLINE
```

Examples:

```vut
return
return 10
```

Semantic analysis validates function return type.

---

## 51. Expressions

Conceptual precedence hierarchy:

```text
expression
  ↓
or_expression
  ↓
and_expression
  ↓
comparison_expression
  ↓
range_expression
  ↓
additive_expression
  ↓
multiplicative_expression
  ↓
unary_expression
  ↓
postfix_expression
  ↓
primary_expression
```

Exact precedence must remain stable once parser tests are established.

---

## 52. Logical OR

```text
or_expression :=
    and_expression { "or" and_expression }
```

---

## 53. Logical AND

```text
and_expression :=
    comparison_expression { "and" comparison_expression }
```

---

## 54. Comparison

```text
comparison_expression :=
    range_expression
    { comparison_operator range_expression }
```

Operators:

```text
==
!=
<
>
<=
>=
```

`is` does not exist.

---

## 55. Range

```text
range_expression :=
    additive_expression
    [ range_operator additive_expression ]

range_operator :=
    ".."
  | "..="
```

Examples:

```vut
0..10
0..=10
```

---

## 56. Additive Operators

```text
additive_expression :=
    multiplicative_expression
    { ("+" | "-") multiplicative_expression }
```

---

## 57. Multiplicative Operators

```text
multiplicative_expression :=
    unary_expression
    { ("*" | "/" | "%") unary_expression }
```

---

## 58. Unary Operators

Current logical unary operator:

```text
not
```

Numeric unary operators may include:

```text
+
-
```

Conceptually:

```text
unary_expression :=
    ("not" | "+" | "-") unary_expression
  | postfix_expression
```

---

## 58a. Await Expression

```text
await_expression :=
    "await" await_operand

await_operand :=
    primary_expression { member_access | call_suffix }
```

Examples:

```vut
await operation()
await one().and_then(other)
await fetch()?
```

`await` is a prefix expression whose operand is a member/call chain. Because
`await_expression` is a primary expression, the surrounding postfix expression
may apply `result_propagation` and further member access after it. Therefore:

```text
await operation()?
```

parses as:

```text
(await operation())?
```

and:

```text
await operation()?.field
```

parses as:

```text
((await operation())?).field
```

`await` and `?` are independent mechanisms. `await` is only valid inside an
`async fn`; `?` requires a `result(T, E)` operand.

The base of an `await` operand is not itself an `await`; nested awaits use
parentheses or ordinary call arguments, for example:

```vut
await combine(await first())
```

`await` never appears as the operand of another `await`.

---

## 59. Postfix Expressions

```text
postfix_expression :=
    primary_expression
    { member_access | call_suffix | result_propagation }

result_propagation := "?"
```

This supports:

```vut
user.name
user.login()
math.add()
value.to_str()
value?
load_user()?.name
```

Call suffixes repeat, so the result of a call may itself be called when it has a
function type:

```vut
get_function()(10)
make_adder(10)(5)
foo()(1)("x")
```

See `04-functions-methods.md` §47b for the call-typing rules.

`result_propagation` is the `?` operator defined by
`specs/result/03-question-operator.md`.

Because `await_expression` is a primary expression, postfix operators apply
after `await`:

```text
await operation()?       ->  (await operation())?
await operation()?.field ->  ((await operation())?).field
```

---

## 60. Member Access

```text
member_access :=
    "." identifier
```

---

## 61. Call

```text
call_suffix :=
    "(" [ argument_list ] ")"
```

---

## 62. Arguments

```text
argument_list :=
    argument { "," argument }

argument :=
    expression
  | identifier "=" expression
  | "..." expression
```

A `...expr` spread argument supplies a `list(T)`, `array(T, N)`, or another
variadic view to a trailing variadic parameter and must be the last argument.

The semantic checker determines whether named arguments are accepted by the target callable.

---

## 63. Primary Expressions

```text
primary_expression :=
    literal
  | identifier
  | parenthesized_expression
  | if_expression
  | match_expression
  | await_expression
```

---

## 64. Parenthesized Expression

```text
parenthesized_expression :=
    "(" expression ")"
```

Parentheses group expressions.

They do not create tuples.

Example:

```vut
value = (a + b) * c
```

---

## 65. No Tuple Grammar

The parser must not interpret:

```text
(a, b)
```

as a tuple.

If comma-separated parenthesized expressions are not valid in another defined construct, they must produce a syntax error.

---

## 66. No Square-Bracket Grammar

The core language does not use:

```text
[]
```

for:

* list literals
* indexing
* slicing
* generic types

Indexing uses methods such as:

```vut
items.at(0)
```

---

## 67. No Brace Blocks

The parser must not accept:

```text
{}
```

as block syntax.

Blocks are indentation-based.

---

## 68. No `while`

`while` is not part of Vut grammar.

Conditional loops use:

```vut
for condition:
  ...
```

---

## 69. No `impl`

`impl` is not part of the interface grammar.

Structural interface satisfaction is semantic and automatic.

---

## 70. No `class`

Vut does not define:

```text
class
extends
inherits
```

State uses `data`.

Behavior uses methods/interfaces/composition.

---

## 71. No Visibility Keywords

Do not define grammar for:

```text
pub
public
private
export
```

Visibility is derived from identifier naming.

---

## 72. Error Recovery

Parser grammar implementation must include recovery points.

Useful synchronization tokens include:

```text
NEWLINE
DEDENT
top-level declaration keywords
```

A syntax error should not necessarily abort the entire file.

---

## 73. Missing Colon Recovery

For:

```vut
if active
  print("yes")
```

the parser should infer that `:` is likely missing and produce a targeted diagnostic rather than generic token failure.

---

## 74. Indentation Recovery

Malformed indentation should produce dedicated diagnostics.

The parser/lexer should recover at a valid dedent or subsequent top-level declaration where possible.

---

## 75. Grammar Stability

Once syntax is implemented and released, changes to grammar must consider:

* source compatibility
* formatter behavior
* parser diagnostics
* language-server behavior
* edition/version policy

Compatibility is defined in:

```text
specs/23-compatibility-versioning.md
```

---

## 75a. Anonymous Functions and Function Types

Lambda/anonymous functions are first-class values. There are two surface
forms and both produce the same anonymous function type.

Single expressions use the arrow form:

```text
lambda_expression :=
    identifier "=>" expression
  | "(" [ identifier { "," identifier } ] ")" "=>" expression
```

Examples:

```vut
x => x * 2
(a, b) => a + b
() => out("Hello")
```

Multi-line bodies use `fn` with an indented block:

```text
lambda_expression :=
    "fn" "(" [ lambda_parameter_list ] ")" block

lambda_parameter_list :=
    lambda_parameter { "," lambda_parameter }

lambda_parameter :=
    identifier [ ":" type_expression ]
```

Example:

```vut
numbers.map(fn(x):
  value = x * 2
  value + 1
)
```

A function type is written:

```vut
fn(int) -> int
fn(str)
fn()
```

Rules:

```text
arrow lambda parameters have no type annotations
fn lambda parameters may be annotated
parameter and return types may be inferred from an expected function type
lambdas are non-capturing; they cannot reference enclosing locals
a named function may be used where a function type is expected
```

---

## 76. Deferred Grammar

The following syntax is intentionally not finalized:

```text
unsafe pointer operations
function-level unsafe
threads
channels
atomics
select
cross-module extension methods
```

The parser must not invent these features.

The following are no longer deferred; their grammar is defined in this document
and their dedicated specifications (and is implemented): `async fn`/`await`
(`specs/async/01-syntax.md`), generic declarations and constraints (`specs/04`,
`specs/06`), payload enum patterns (`specs/05`), static methods and same-module
extension methods (`specs/04`), `spawn`/`vut(...)` (`specs/vutcom/`), optional
narrowing (`specs/optional/`), and result construction (`specs/result/`).

---

## 77. Grammar Principles

1. Vut is indentation-based.
2. Newlines terminate statements.
3. Blocks use `:`.
4. Semicolons are not required.
5. Lists use `@()`.
6. Parentheses group expressions but do not create tuples.
8. Generic type application uses parentheses.
9. `for` is the only loop keyword.
10. `is` does not exist.
11. `impl` does not exist.
12. Visibility keywords do not exist.
13. Methods do not declare `self`.
14. Imports use `import`, `as`, and `at`.
15. Parser recovery is mandatory.
16. Deferred syntax must not be guessed.
17. `async` is a function/method modifier.
18. `await` is a prefix expression valid only inside `async fn`.
19. `await` binds tighter than postfix `?`.

This document is normative for Vut's formal grammar direction.
