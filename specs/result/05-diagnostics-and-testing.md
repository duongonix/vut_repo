# Result Diagnostics and Testing

The compiler must diagnose:

- `ok(...)` or `err(...)` without an expected `result(T, E)`
- constructor payload type mismatch
- applying `?` to a non-result value
- using `?` in a function that does not return `result(_, E)`
- `?` error type mismatch
- non-exhaustive result matches
- result patterns used against non-result values
- enum patterns used against result values
- `result(T, E)` in FFI v1 extern signatures

Required implementation coverage:

- parser tests for `ok(...)`, `err(...)`, result match patterns, and postfix `?`
- semantic tests for contextual constructors, propagation, exhaustive matching,
  and FFI rejection
- MIR tests for explicit construct/state/payload operations and propagation CFG
- codegen tests through existing native branch, aggregate, and ownership paths
- full `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test`

