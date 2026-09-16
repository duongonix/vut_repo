# Phase 01 — Workspace and Compiler Foundation

## Status

Complete

## Goal

Create the modular, global-state-free foundation for compiler, runtime, and tooling.

## Tasks

- [x] Create separate source, diagnostics, compiler, runtime, CLI, and VPM crates.
- [x] Implement session/configuration, source IDs, spans, UTF-8 loading, and line indexes.
- [x] Provide an initial structured diagnostic model.
- [x] Establish remaining Phase 01 coverage.

## Tests

- [x] Deterministic IDs, UTF-8 location mapping, invalid byte boundaries, and structured diagnostics.
- [x] File-loading and multi-file integration coverage.

## Benchmarks

None yet.

## Completed Work

Source locations use a precomputed line-start index and binary search. Compiler state is session-local.

## Decisions

`vut-source` owns source data/spans; diagnostics depends on it; compiler composes both. Runtime, CLI, and VPM are separate from compiler core.

## Known Limitations

Rich terminal/machine diagnostic rendering remains scheduled for Phase 04.
