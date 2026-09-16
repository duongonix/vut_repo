//! Standard-stream primitives (`vut_rt_io_*`).
//!
//! Text output to standard output is already provided by the core runtime
//! (`print`/`out`). This module adds the remaining stream primitives that the
//! high-level `io` module needs. All fallible operations use the shared result
//! envelope.
use std::io::Write as _;

use vut_runtime::abi::ManagedString;
use vut_runtime::bytes::ManagedBytes;

use crate::abi;

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed string handle.
pub unsafe extern "C" fn vut_rt_io_stderr_write_v1(
    value: *const ManagedString,
) -> *mut ManagedBytes {
    let Some(value) = (unsafe { abi::text(value) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid text");
    };
    let mut stderr = std::io::stderr();
    match stderr
        .write_all(value.as_bytes())
        .and_then(|()| stderr.flush())
    {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_io_flush_v1() -> *mut ManagedBytes {
    match std::io::stdout().flush() {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}
