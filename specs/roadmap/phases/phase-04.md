# Phase 04 — Diagnostics System

## Status

Complete

## Goal

Provide one structured diagnostic model for compiler stages and human/machine renderers.

## Tasks

- [x] Model severity, stable codes, primary/secondary/related labels, notes,
  help, and structured expected/found values.
- [x] Render plain and color-aware source frames through `SourceManager`.
- [x] Emit a versioned machine-readable JSON representation.
- [x] Sort diagnostics deterministically and suppress dependent cascades.
- [x] Provide precise template lexer/parser diagnostics.

## Tests

- [x] Cover structured fields, rendering modes, machine output, deterministic
  ordering, cascade suppression, multiple-error recovery, and template errors.

## Benchmarks

Diagnostics are a cold error path; line lookup uses the Phase 01 line index.

## Completed Work

Compiler stages emit data rather than ANSI strings. Plain, color, and JSON
rendering share the same diagnostic structure and source lookup API.

## Decisions

Diagnostic codes use a compact strongly represented value and a centralized
registry. Diagnostic construction accepts only registered code values, not
arbitrary strings. Rendering remains outside lexer/parser stages.

## Known Limitations

IDE protocol adapters can consume the machine representation in a later tooling phase.
