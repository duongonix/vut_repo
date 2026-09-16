---
title: Compiler
description: 'A native compiler with a shared frontend for the toolchain.'
section: Tooling
order: 7
---

## Pipeline

Vut compilation separates lexical analysis, parsing, module resolution, type checking, intermediate representations, optimization, and native code generation. Each stage owns its own diagnostics and representation.

```text
Source → Lexer → Parser → Resolver → Type checking
       → MIR → Optimization → Native code
```

## Static guarantees

Normal bindings have fixed inferred or annotated types. Incompatible assignments fail at compile time rather than silently becoming dynamic values. Conditions require `bool` and module privacy is enforced by the resolver.

## Tooling boundary

The CLI is a thin wrapper over reusable compiler libraries. VPM orchestrates projects and packages; the language server reuses the frontend rather than implementing another set of language rules.

## Native, not browser execution

Use [Vut CLI](/docs/tooling/vut-cli/) to build or run locally. The website playground edits and exports examples; it does not contain a browser compiler or pretend to execute programs.
