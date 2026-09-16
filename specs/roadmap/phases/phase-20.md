# Phase 20 — Tooling, Testing and MVP Hardening

## Status

Complete

VPM now provides the finalized `test`, `fmt`, `lint`, `doc`, `clean`, `tree`,
`outdated`, `search`, and `info` commands. Publishing and authentication remain
deliberately unavailable as required by the MVP roadmap.

The reusable `vut-tooling` library discovers project sources, validates them
through the normal lexer/parser, applies deterministic two-space formatting,
and emits conservative `W####` lint findings. Formatting is syntax-safe,
idempotent, supports check mode, preserves comments and string contents, and
never edits malformed source.

The test runner discovers standalone programs below `tests/`, builds each with
the production compiler pipeline, executes them in isolation, filters them by
stable path, and produces a deterministic pass/fail summary. This avoids
inventing test syntax that the language grammar has not finalized.

Documentation generation consumes existing `###` doc comments. Cleaning is
restricted to project build/cache outputs and preserves sources, `vpm.toml`,
`vpm.lock`, and the global package store. Dependency and package metadata
commands use deterministic lockfile/provider data.

The required MVP integration program is maintained at
`examples/mvp_integration.vut` and is compiled and executed by the native
compiler integration suite, including input, data construction, an instance
method, template interpolation, a typed list, and value/index iteration.

MVP hardening additionally requires every type-correct MVP operator to reach a
native lowering path rather than an `Unsupported` escape hatch. Floating-point
remainder uses the versioned runtime ABI; impossible typed-MIR combinations are
reported as internal backend invariant violations. Native compiler/linker tests
run on Windows, Linux, and macOS through a portable Rust startup shim.

CI builds and exercises the runtime ABI on all three host families. Linux also
runs the runtime and ownership suites under AddressSanitizer and LeakSanitizer.
Stress coverage repeatedly clones and mutates COW lists/maps and performs
twenty thousand managed-string allocation/transform/release cycles, requiring
the live allocation count to return to its exact baseline.

The formatter consumes a dedicated lossless syntax tree whose tokens cover and
reconstruct every source byte, including whitespace, CRLF, comments, multiline
comments, strings, and templates. Validation, syntax capture, canonical printing,
discovery, and linting are separate modules. Formatting is idempotent and never
rewrites string or comment contents.

The standalone-program test framework filters deterministically, isolates build
directories, captures stdout/stderr, reports failed output, and continues across
independent files. The standard testing module exposes structured `assert`,
`assert_eq`, and `assert_ne` results with expected/found values. A dedicated
`vut-fuzz` crate runs saved corpus entries plus deterministic generated UTF-8 and
malformed syntax through lexer, parser, and lossless reconstruction; CI runs a smoke
corpus on Linux changes and one million cases daily.

Package hardening makes manifest schemas strict, gives lockfiles an explicit
compatibility range, validates identities/revisions/BLAKE3 checksums, and checks
remote snapshot checksum, manifest identity, source layout, UTF-8, revision, and
path safety before a staging directory can become an installed package.
