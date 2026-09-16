# Phase 02 — Lexer

## Status

Complete

## Goal

Implement the complete allocation-conscious, indentation-aware Vut lexer.

## Tasks

- [x] Establish token and lexer crate boundaries.
- [x] Preserve byte spans instead of duplicating token text.
- [x] Implement initial keywords, numbers, operators, strings, and indentation.
- [x] Complete comments, template segments, recovery, and all required syntax.

## Tests

- [x] Initial indentation, range, and invalid-character coverage.
- [x] Complete the Phase 02 lexer matrix and malformed-input regression tests.

## Benchmarks

The release-mode throughput benchmark scans a representative template-heavy
source corpus. The current local baseline is approximately 90 MiB/s.

## Completed Work

The lexer performs a forward source scan and emits compact `TokenKind` plus
`Span` values. It supports indentation, delimiter-aware newlines, all locked
keywords/operators/literals, comments, relative-import dots, recoverable
errors, and statically tokenized template interpolation.

## Decisions

Vut-specific indentation and template diagnostics justify a custom lexer.

## Known Limitations

Expression syntax inside interpolation is validated by the Phase 03 parser;
the lexer supplies ordinary expression tokens with exact source spans.
