# Phase 15 — Standard Library MVP

## Status

Complete

`vut-stdlib` exposes core, typed collections, UTF-8 string operations, exact
`print`/`out`/`input` behavior, typed filesystem errors, environment, math,
Duration/Instant, Option, and Result. The compiler binds the console built-ins,
type-checks interpolations, lowers every template segment, and emits versioned
runtime calls without runtime expression parsing.

Verified by standard-library unit tests and native template/I/O integration tests.
