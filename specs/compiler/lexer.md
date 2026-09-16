# specs/compiler/lexer.md

# Vut Lexer

## 1. Purpose

The lexer converts UTF-8 Vut source code into a token stream consumed by the parser.

Conceptually:

```text
SourceFile
    ↓
Lexer
    ↓
TokenStream
```

The lexer is responsible for lexical structure only.

It must not:

```text
type-check
resolve names
resolve modules
validate interfaces
perform code generation
```

---

## 2. Goals

The lexer must be:

```text
fast
deterministic
allocation-conscious
UTF-8 correct
span-preserving
error-recoverable
indentation-aware
```

Lexer design must follow:

```text
specs/01-language-syntax.md
specs/09-errors-diagnostics.md
specs/21-grammar.md
specs/22-error-codes.md
specs/24-rules.md
```

---

## 3. Source Representation

The lexer receives:

```text
SourceId
source text
```

It should not repeatedly read source files from disk.

Source loading belongs to the source manager.

Conceptually:

```rust
Lexer {
    source_id,
    source,
    cursor,
    indentation_state,
}
```

Exact Rust representation is implementation-specific.

---

## 4. Token Representation

Each token should contain at minimum:

```text
TokenKind
Span
```

Conceptually:

```text
Token
├── kind
└── span
```

Avoid storing duplicate owned source strings in every token.

Identifiers, strings and numbers should normally refer to source spans and be decoded only when necessary.

---

## 5. Span

A span identifies a source range.

Conceptually:

```text
Span
├── source_id
├── start
└── end
```

Offsets should preferably use byte offsets for efficient UTF-8 source slicing.

All spans must end on valid UTF-8 boundaries.

---

## 6. Token Categories

Token kinds should include categories for:

```text
identifiers
keywords
numbers
strings
operators
punctuation
newlines
indentation
end-of-file
errors/recovery
```

---

## 7. Keywords

Current Vut keywords include:

```text
fn
data
interface
enum
type

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

Future/deferred words must not automatically become reserved tokens.

Only reserve them when the language specification finalizes them.

---

## 8. Operators

Lexer must recognize:

```text
+
-
*
/
%

==
!=
>
<
>=
<=

=
:

->
..

..=
?
.
,
```

Ordering matters.

For example:

```text
..=
```

must not tokenize as:

```text
..
=
```

Likewise:

```text
>=
```

must not tokenize as:

```text
>
=
```

Use longest-valid-token matching.

---

## 9. Special Literal Prefixes

Vut uses:

```text
@(
```

for lists. `$` is only valid inside a string literal, where it begins template
interpolation (`$identifier` or `$(expression)`) unless escaped as `\$`.

The lexer may represent list syntax as:

```text
At + LParen
```

or dedicated compound tokens.

The parser API should remain clean regardless of the internal choice.

Do not duplicate semantic meaning in the lexer unnecessarily.

The unified method header `fn Type.method(...)` is tokenized as the ordinary
`fn`, identifier, dot, identifier, and delimiter tokens. Whether the declaration
is a free function or method is a parser decision.

---

## 10. Parentheses

Recognize:

```text
(
)
```

Vut does not currently use:

```text
[
]
{
}
```

for core syntax.

Unsupported punctuation should produce precise lexical/parser diagnostics rather than silently being accepted.

---

## 11. Identifier Lexing

Identifiers are case-sensitive.

Conceptually:

```text
identifier_start:
  letter
  _

identifier_continue:
  letter
  digit
  _
```

Exact Unicode identifier policy should be explicitly defined before widening beyond the initial supported identifier set.

Use proven Unicode facilities if full Unicode identifiers are introduced.

---

## 12. Private Names

The lexer does not determine visibility.

Example:

```text
_token
```

is lexed as an identifier.

Semantic analysis later determines that leading `_` means private.

---

## 13. Constants

The lexer does not classify ALL-CAPS identifiers as constants.

Example:

```text
MAX_SIZE
```

remains an identifier token.

Semantic/name analysis determines constant behavior.

---

## 14. Integer Literals

Initial integer literal support:

```text
0
10
123456
```

The lexer should preserve the original span and avoid immediately forcing all numbers into one machine integer type.

Overflow validation belongs to numeric literal/type analysis.

Malformed numbers must produce:

```text
E0004
```

or the appropriate numeric lexical diagnostic.

---

## 15. Floating-Point Literals

Initial examples:

```text
1.0
3.14
100.25
```

The lexer must distinguish:

```text
1.0
```

from range syntax such as:

```text
1..10
```

Correct tokenization of:

```text
1..10
```

should conceptually be:

```text
Integer(1)
RangeExclusive
Integer(10)
```

not a malformed floating number.

---

## 16. Strings

Canonical Vut strings use:

```text
"hello"
```

Lexer is responsible for:

```text
finding closing quote
validating escape syntax
handling escaped quotes
producing precise spans
```

Unterminated string:

```text
E0002
```

Invalid escape:

```text
E0003
```

Do not panic on malformed strings.

---

## 17. Comments

Single-line comment:

```vut
# comment
```

continues until newline or EOF.

Multi-line comment:

```vut
##
comment
##
```

Doc comment direction:

```vut
### documentation
```

Comments should be represented in a way that allows formatter/documentation tooling to preserve or inspect them.

If the semantic AST does not need comments, a lossless syntax representation or side table may retain them.

---

## 18. Comment Priority

Lexer must correctly distinguish:

```text
#
##
###
```

according to language syntax.

Do not tokenize `##` as two independent line comments.

---

## 19. Newlines

Newlines are significant because Vut is indentation-based.

The lexer should emit logical newline information.

Conceptually:

```text
NEWLINE
```

Blank/comment-only lines must not create incorrect block structure.

---

## 20. Indentation

Vut formatter canonical indentation is:

```text
2 spaces
```

The lexer/parser must detect indentation structure.

A practical token model may emit:

```text
INDENT
DEDENT
```

Example:

```vut
if active:
  print("yes")
  login()
print("done")
```

may conceptually become:

```text
IF IDENT COLON NEWLINE
INDENT
IDENT ...
NEWLINE
IDENT ...
NEWLINE
DEDENT
IDENT ...
NEWLINE
EOF
```

---

## 21. Indentation Stack

Maintain an indentation stack.

Conceptually:

```text
[0]
```

Entering a deeper block:

```text
[0, 2]
```

Nested:

```text
[0, 2, 4]
```

Dedent must return to an existing indentation level.

Invalid dedent should produce:

```text
E0107
```

or equivalent parser-facing indentation diagnostic.

---

## 22. Tabs

Canonical indentation uses spaces.

Tabs in indentation should be rejected or normalized only according to explicitly defined language behavior.

Do not silently allow ambiguous mixed tabs/spaces.

Diagnostics should clearly explain the indentation found.

---

## 23. Delimiter-Aware Newlines

Inside parenthesized constructs such as:

```vut
User(
  name = "Ha",
  age = 20
)
```

or:

```vut
@(
  1,
  2,
  3
)
```

newlines should not terminate expressions in the same way as top-level statement newlines.

Lexer/parser should track delimiter nesting.

---

## 24. EOF Dedents

At EOF, remaining open indentation levels should produce required `DEDENT` events before EOF where the token model requires it.

---

## 25. Error Recovery

Invalid characters must produce a recoverable token/diagnostic.

Example:

```text
E0001
```

The lexer should advance past the invalid sequence safely and continue where possible.

Never enter an infinite loop on malformed input.

---

## 26. Performance

Lexer should be approximately linear in source length:

```text
O(n)
```

Avoid:

```text
repeated substring allocation
regex execution per character
restarting scans
cloning source fragments
```

---

## 27. Library Choice

A crate such as:

```text
logos
```

may be evaluated.

However, Vut's indentation-sensitive and diagnostic-heavy requirements may justify a custom lexer.

If custom:

```text
keep it modular
benchmark it
fuzz it
test malformed input
```

Do not choose either approach blindly.

---

## 28. Testing

Required test categories:

```text
identifiers
keywords
operators
longest-match operators
integers
floats
range ambiguity
strings
escapes
comments
multi-line comments
indentation
nested indentation
invalid dedent
blank lines
comment-only lines
UTF-8
EOF
invalid characters
```

---

## 29. Fuzzing

Lexer fuzzing should ensure arbitrary bytes converted/validated as source input cannot cause:

```text
panic
out-of-bounds access
infinite loop
memory corruption
```

---

## 30. Rules

1. Lexer performs lexical analysis only.
2. Tokens preserve exact source spans.
3. Avoid token-owned duplicate strings.
4. Indentation is first-class.
5. Longest token wins for overlapping operators.
6. Malformed source must not panic.
7. Blank/comment lines must not corrupt indentation.
8. UTF-8 boundaries must remain correct.
9. Lexer should run approximately O(n).
10. Deferred syntax must not be lexically promoted without specification.



