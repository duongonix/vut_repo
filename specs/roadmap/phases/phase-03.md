# Phase 03 — Parser and AST

## Status

Complete

## Goal

Implement the recovering Vut parser and span-preserving syntax AST.

## Tasks

- [x] Implement modular recursive-descent declaration and statement parsing.
- [x] Implement explicit Pratt precedence and postfix parsing.
- [x] Parse every locked declaration, control-flow form, type form, and literal.
- [x] Parse both free functions and instance methods from the unified `fn`
  declaration prefix and reject the removed method syntax without compatibility.
- [x] Parse template strings into text and ordinary expression segments.
- [x] Reject tuple syntax and recover at declaration/line boundaries.

## Tests

- [x] Cover declarations, expressions, precedence, calls, types, templates,
  control flow, imports, targeted diagnostics, and recovery.

## Benchmarks

The release parser benchmark establishes a local baseline of approximately
13 MiB/s on template-heavy function input.

## Completed Work

The AST retains byte spans for meaningful nodes. The parser is split into
declaration, statement/control-flow, and expression modules and never performs
name or type resolution.

## Decisions

Calls remain syntactic so later resolution decides whether a callee is a
function or data constructor. Template expressions consume the normal Pratt
parser token stream and are never parsed at runtime.

Method AST nodes store the receiver type and method name separately. The
qualified `fn Type.method(...)` header is the sole implemented method syntax.

## Known Limitations

Semantic validation belongs to Phase 05 and later phases. Rich diagnostic
rendering belongs to Phase 04.
