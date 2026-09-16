# Phase 16 — Vut CLI

## Status

Complete

The `vut` executable provides exactly `run` and `build`, supports project and
standalone-file entry points, debug/release output, explicit target/output,
program arguments after `--`, color policy, human/JSON diagnostics, help,
version output, and non-zero failure propagation. Compilation remains in
`vut-compiler`; the CLI only coordinates it.

Verified by CLI contract tests plus the compiler's native build/run tests.
