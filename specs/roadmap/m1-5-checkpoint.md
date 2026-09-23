# M1.5 LSP checkpoint

## Status

In Progress

## Phase 1 — Project Analysis Foundation

Complete.

- LSP discovers the nearest VPM project and analyzes its full source graph.
- Installed dependency roots come from the canonical VPM lock/store model.
- The compiler frontend owns source discovery, overlays, stdlib injection,
  resolver, target-aware type checking, HIR and MIR diagnostics.
- Open documents overlay disk modules without changing their stable `SourceId`.
- Standalone source directories remain supported when no VPM manifest exists.
- The LSP initialization option `target` selects the compiler target configuration.

## Remaining phases

## Phase 2 — Diagnostics Parity

Complete.

- Diagnostics come from the canonical project compiler pipeline through MIR.
- Stable compiler codes, severity, primary ranges, expected/found, notes and help
  are preserved in LSP diagnostics.
- Secondary and related compiler labels become LSP related locations, including
  cross-file locations.
- UTF-8 byte spans are converted to UTF-16 ranges using each source's overlay text.
- A change republishes diagnostics for every open document in the affected project.
- Published diagnostics carry the matching LSP document version.

## Remaining phases

## Phase 3 — Shared Semantic Query Layer

Complete.

- Resolver-backed symbol-at-position, declaration, reference and workspace
  symbol queries are centralized in one semantic query module.
- Queries are source-aware and use stable `SourceId` values across overlays.
- Existing hover and rename symbol lookup now reuse the shared query layer.

## Phase 4 — Cross-file Navigation and Symbols

Complete.

- Go-to-definition resolves declarations in project modules, installed
  dependencies and stdlib sources.
- References return locations across all analyzed modules and honor the client's
  `includeDeclaration` flag.
- Workspace symbols are advertised and searched from the resolved project graph.
- Document symbols include extern functions, interface methods, enum variants
  and enum payload fields.

## Phase 5 — Hover, Completion and Signature Help

Complete.

- Hover presents source-level Vut declarations and canonical semantic types.
- Completion is lexical-scope aware and supports module exports, data fields,
  instance methods, static methods and builtin methods.
- Signature help uses compiler-resolved call targets for functions, methods,
  constructors and extern calls, with builtin-method fallback.
- Active parameters account for nested calls and collection literals.
- Tooling answers use the project graph, open-document overlays and compiler
  type information established by Phases 1–3.

## Phase 6 — Safe Workspace Rename

Complete.

- Prepare-rename and rename use resolver symbol identity rather than text search.
- Declaration and reference edits span every affected project module with
  source-specific UTF-16 ranges.
- Invalid identifiers, keywords, namespace collisions, synthetic declarations,
  stdlib symbols and dependency-owned symbols are rejected before edits are made.
- Rename is atomic: an uneditable affected source rejects the whole operation
  instead of producing a partial workspace edit.

## Remaining phases

1. Protocol hardening and final M1.5 tests.
