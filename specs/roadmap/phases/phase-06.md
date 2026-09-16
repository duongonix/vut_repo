# Phase 06 — HIR

## Status

Complete

## Completed Work

Added `vut-hir` with compact typed IDs, resolved module imports and declaration
symbols, backend-independent function/data/interface/enum/type-alias nodes,
source spans, and normalized methods with structured receivers and injected
`self` IDs. Semantic expression types are maintained in a side table.

## Tests

- [x] AST-to-HIR lowering, receiver normalization, implicit `self`, resolved IDs,
  import graph reuse, and span preservation.

## Known Limitations

Machine layouts and ownership operations intentionally remain later-phase IR work.
