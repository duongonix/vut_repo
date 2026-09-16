# Phase 05 — Module and Name Resolution

## Status

Complete

## Goal

Implement modules, imports, lexical scopes, privacy, and dependency source roots.

## Tasks

- [x] Discover every `.vut` module below an explicit source root.
- [x] Index project and dependency roots by structured logical module paths.
- [x] Resolve absolute, aliased, selected, and Vut-relative imports.
- [x] Reject missing modules/symbols, private imports, collisions, and root escape.
- [x] Build a deterministic module graph and dependency-first compile order.
- [x] Detect import cycles and report the complete cycle path.
- [x] Collect module declarations before resolving function and block scopes.
- [x] Resolve parameters, locals, implicit method `self`, and module members.
- [x] Validate method receiver types and build structured per-receiver method
  namespaces with duplicate-method detection.
- [x] Integrate source-root resolution with `CompilerSession`.

## Tests

- [x] Cover absolute imports, aliases, selected imports, `.`, `..`, and `...`.
- [x] Cover privacy, missing targets, collisions, root escape, and full cycles.
- [x] Cover nested scopes, typo suggestions, dependency roots, deterministic
discovery, and dependency-first ordering.
- [x] Cover method receiver association, same-name methods on different types,
  unknown receivers, duplicate methods, and implicit `self`.
- [x] Pass workspace tests, strict Clippy, and formatting checks.

## Completed Work

`vut-resolver` owns module discovery, stable module/symbol IDs, per-module symbol
tables, import normalization, graph analysis, and lexical name binding. Filesystem
paths remain metadata; semantic lookup uses normalized logical module paths.

## Decisions

Visibility is derived centrally from the leading `_` convention. Resolution uses
declaration collection followed by import/body binding so later declarations are
available without coupling parser behavior to semantic order.

## Known Limitations

Incremental graph invalidation and serialized resolver caches remain Phase 19 work.
