//! Standard-stream primitives (`vut_rt_io_*`).
//!
//! Text output to standard output is already provided by the core runtime
//! (`print`/`out`). This module adds the remaining stream primitives that the
//! high-level `io` module needs. All fallible operations use the shared result
//! envelope for data/errors and exact scalar resource replies for byte counts.
use std::{ffi::c_void, io::Write as _};

use vut_runtime::bytes::ManagedBytes;

use crate::abi;

/// Reads one bounded chunk from standard input.
///
/// Success payload is the bytes read; an empty payload means end of input.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_io_stdin_read_v1() -> *mut ManagedBytes {
    read_chunk(&mut std::io::stdin())
}

pub(crate) fn read_chunk(reader: &mut impl std::io::Read) -> *mut ManagedBytes {
    const CHUNK: usize = 64 * 1024;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        match reader.read(&mut buffer) {
            Ok(read) => return abi::ok(&buffer[..read]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return abi::err(abi::io_status(&error), &error.to_string()),
        }
    }
}

pub(crate) fn write_bytes(writer: &mut impl std::io::Write, bytes: &[u8]) -> *mut c_void {
    crate::count::reply(
        writer
            .write_all(bytes)
            .map(|()| u64::try_from(bytes.len()).expect("usize fits u64")),
    )
}

#[unsafe(no_mangle)]
/// # Safety
/// value must be null or a live managed bytes handle.
pub unsafe extern "C" fn vut_rt_io_stdout_write_v2(value: *const ManagedBytes) -> *mut c_void {
    let Some(bytes) = (unsafe { abi::bytes(value) }) else {
        return crate::count::invalid("invalid bytes");
    };
    write_bytes(&mut std::io::stdout(), bytes)
}

#[unsafe(no_mangle)]
/// # Safety
/// value must be null or a live managed bytes handle.
pub unsafe extern "C" fn vut_rt_io_stderr_write_v2(value: *const ManagedBytes) -> *mut c_void {
    let Some(bytes) = (unsafe { abi::bytes(value) }) else {
        return crate::count::invalid("invalid bytes");
    };
    let mut stderr = std::io::stderr();
    crate::count::reply(
        stderr
            .write_all(bytes)
            .and_then(|()| stderr.flush())
            .map(|()| u64::try_from(bytes.len()).expect("usize fits u64")),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_io_flush_v1() -> *mut ManagedBytes {
    match std::io::stdout().flush() {
        Ok(()) => abi::ok(&[]),
        Err(error) => abi::err(abi::io_status(&error), &error.to_string()),
    }
}
