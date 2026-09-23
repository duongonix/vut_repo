//! Panic isolation for the native C ABI boundary.
//!
//! Native code must not unwind across the C ABI boundary: a Rust panic reaching
//! a Vut or C caller is undefined behavior. `or_abort` converts a `Result` into
//! an abort with a diagnostic instead of a panic, so a broken native environment
//! stops the process cleanly rather than unwinding through `extern "C"`.

/// Returns the `Ok` value or aborts the process with `context` on `Err`.
pub(crate) fn or_abort<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("vut native runtime: {context}: {error}");
            std::process::abort();
        }
    }
}
