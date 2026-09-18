//! Standard-stream primitives (`vut_rt_io_*`).
//!
//! Text output to standard output is already provided by the core runtime
//! (`print`/`out`). This module adds the remaining stream primitives that the
//! high-level `io` module needs. All fallible operations use the shared result
//! envelope; read/write counts are encoded as little-endian `i64` payloads.
use std::io::{Read as _, Write as _};

use vut_runtime::bytes::ManagedBytes;

use crate::abi;

/// Reads one bounded chunk from standard input.
///
/// Success payload is the bytes read; an empty payload means end of input.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_io_stdin_read_v1() -> *mut ManagedBytes {
    const CHUNK: usize = 64 * 1024;
    let mut buffer = vec![0_u8; CHUNK];
    let mut stdin = std::io::stdin();
    loop {
        match stdin.read(&mut buffer) {
            Ok(read) => return abi::ok(&buffer[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return abi::err(abi::io_status(&error), &error.to_string()),
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed bytes handle.
pub unsafe extern "C" fn vut_rt_io_stdout_write_v1(
    value: *const ManagedBytes,
) -> *mut ManagedBytes {
    let Some(bytes) = (unsafe { abi::bytes(value) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid bytes");
    };
    let mut stdout = std::io::stdout();
    match stdout.write_all(bytes) {
        Ok(()) => abi::ok(&(bytes.len() as i64).to_le_bytes()),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or a live managed bytes handle.
pub unsafe extern "C" fn vut_rt_io_stderr_write_v1(
    value: *const ManagedBytes,
) -> *mut ManagedBytes {
    let Some(bytes) = (unsafe { abi::bytes(value) }) else {
        return abi::err(abi::INVALID_INPUT, "invalid bytes");
    };
    let mut stderr = std::io::stderr();
    match stderr.write_all(bytes).and_then(|()| stderr.flush()) {
        Ok(()) => abi::ok(&(bytes.len() as i64).to_le_bytes()),
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
