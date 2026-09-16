
# Vut Compiler Diagnostics

## 1. Purpose

This document defines the compiler-internal diagnostic architecture.

User-facing layout follows:

```text
specs/09-errors-diagnostics.md
specs/22-error-codes.md
```

This document focuses on implementation structure.

Project/module graph diagnostics include:

```text
E3008 module namespace conflict
```

Use `E3008` when a local top-level module namespace matches a dependency import
namespace. The diagnostic should include the local module path, dependency
namespace, and help text recommending a dependency alias.

---

## 2. Diagnostic Is Structured Data

Never represent compiler diagnostics only as:

```rust
String
```

Conceptually:

```text
Diagnostic
├── severity
├── code
├── title
├── primary
├── secondary labels
├── notes
├── help
└── metadata
```

---

## 3. Severity

Use enum:

```text
Error
Warning
Note
Help
```

Primary diagnostic normally has:

```text
Error
Warning
```

while notes/help attach to it.

---

## 4. Error Code

Use strongly represented code values.

Conceptually:

```text
ErrorCode
WarningCode
```

Do not spread arbitrary `"E1003"` strings throughout the compiler.

---

## 5. Labels

A diagnostic label should contain:

```text
Span
message
style/role
```

Primary label identifies the main source error.

Secondary labels provide related context.

---

## 6. Related Locations

Example:

```text
primary:
  invalid argument

related:
  parameter declared here
```

Diagnostic model must support multiple source files.

---

## 7. Expected/Found

Type diagnostics should store structured:

```text
expected
found
```

where possible.

Renderer converts semantic values into readable text.

Do not pre-render them too early.

---

## 8. Help

Help messages should only be produced when compiler has a reliable correction or direction.

Avoid misleading suggestions.

---

## 9. Renderer Separation

Compiler stages produce:

```text
Diagnostic
```

Renderer produces:

```text
terminal text
plain text
JSON
future IDE protocol
```

Do not let lexer/type checker directly emit ANSI codes.

---

## 10. Terminal Renderer

Terminal renderer should support:

```text
color auto
color always
color never
```

according to CLI configuration.

---

## 11. Syntax Highlighting

Source snippets may highlight:

```text
keywords
strings
numbers
error spans
```

Rendering implementation should remain independent from semantic diagnostics.

---

## 12. Machine-Readable Output

Structured output should include at least:

```text
severity
code
message
file
start offset/line/column
end offset/line/column
labels
notes
help
```

Exact JSON schema should be versionable.

---

## 13. Source Manager Integration

Renderer obtains:

```text
source text
line offsets
path
```

through source manager.

Compiler phases should only need spans.

---

## 14. Line Lookup

Do not rescan the source from the start for every diagnostic.

Source manager should maintain line-start indexes.

---

## 15. Diagnostic Sink

Compiler session may collect diagnostics through:

```text
DiagnosticSink
```

or equivalent.

Avoid process-global diagnostic state.

---

## 16. Recovery

Diagnostic system should support many errors per compilation.

Do not terminate compilation immediately after first recoverable error.

---

## 17. Cascade Suppression

Later phases must recognize invalid/error nodes/types.

Example:

Unknown symbol:

```text
E2001
```

should not automatically create five derivative type errors.

---

## 18. Deterministic Ordering

Diagnostics must have stable order.

Recommended sorting dimensions:

```text
source/module order
span start
severity/code tie-breaker
```

Parallel compilation must not produce nondeterministic output ordering.

---

## 19. Maximum Error Count

Compiler may impose a reasonable maximum diagnostic count to avoid pathological output.

If limit is reached, report that additional diagnostics were suppressed.

Do not allow malformed source to generate millions of errors.

---

## 20. ICE Diagnostics

Internal compiler errors must be distinguished from source errors.

Include useful internal context without dumping sensitive or irrelevant process state.

---

## 21. External Libraries

A rendering library such as:

```text
codespan-reporting
ariadne
annotate-snippets
```

may be used.

Vut's internal `Diagnostic` model should not be tightly coupled to the library.

---

## 22. Linter Reuse

Warnings from linter should use the same diagnostic infrastructure.

---

## 23. VPM Reuse

VPM may reuse styling/rendering primitives.

However, package/network errors should not fake source spans.

---

## 24. Tests

Snapshot/golden tests must cover:

```text
type mismatch
unknown symbol
private access
module missing
interface mismatch
missing colon
indentation
multiple files
related location
plain mode
colored mode where stable
JSON output
multiple diagnostics ordering
```

---

## 25. Rules

1. Diagnostics are structured objects.
2. Rendering is separate.
3. Spans remain source-manager based.
4. Support multiple labels/files.
5. Error codes are stable types.
6. Avoid cascades.
7. Ordering is deterministic.
8. Terminal and machine formats use same semantic diagnostic.
9. Compiler phases never embed ANSI styling.
10. User source errors never cause panics.
