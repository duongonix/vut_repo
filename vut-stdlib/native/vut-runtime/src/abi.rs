//! Shared native result ABI for the official Vut standard library.
//!
//! Fallible primitives return a `bytes` *envelope*:
//!
//! ```text
//! [status: i32 little-endian][payload...]
//! ```
//!
//! `status == 0` means success and the payload is the operation's result.
//! Otherwise the payload is a human-readable UTF-8 diagnostic message. The Vut
//! layer (`_internal/abi.vut`) decodes the envelope and maps the status code
//! into a domain error kind, so no platform error object and no global mutable
//! state ever crosses the ABI. Primitives that cannot fail return their value
//! directly.
use vut_runtime::abi::{ManagedString, managed_string};
use vut_runtime::bytes::{ManagedBytes, managed_bytes};

pub const OK: i32 = 0;
pub const NOT_FOUND: i32 = 1;
pub const PERMISSION_DENIED: i32 = 2;
pub const ALREADY_EXISTS: i32 = 3;
pub const INVALID_INPUT: i32 = 4;
pub const NOT_A_DIRECTORY: i32 = 5;
pub const IS_A_DIRECTORY: i32 = 6;
pub const DIRECTORY_NOT_EMPTY: i32 = 7;
pub const UNSUPPORTED: i32 = 8;
pub const INTERRUPTED: i32 = 9;
pub const UNEXPECTED_EOF: i32 = 10;
pub const INVALID_DATA: i32 = 11;
pub const BROKEN_PIPE: i32 = 12;
pub const TIMED_OUT: i32 = 13;
pub const OTHER: i32 = 99;

/// Builds an envelope from a status code and raw payload.
#[must_use]
pub fn envelope(status: i32, payload: &[u8]) -> *mut ManagedBytes {
    let mut buffer = Vec::with_capacity(4 + payload.len());
    buffer.extend_from_slice(&status.to_le_bytes());
    buffer.extend_from_slice(payload);
    managed_bytes(buffer)
}

/// Successful envelope with raw payload.
#[must_use]
pub fn ok(payload: &[u8]) -> *mut ManagedBytes {
    envelope(OK, payload)
}

/// Successful envelope whose payload is UTF-8 text.
#[must_use]
pub fn ok_text(value: &str) -> *mut ManagedBytes {
    envelope(OK, value.as_bytes())
}

/// Failed envelope carrying a human-readable message.
#[must_use]
pub fn err(status: i32, message: &str) -> *mut ManagedBytes {
    envelope(status, message.as_bytes())
}

/// Maps a `std::io::ErrorKind` to a stable status code.
#[must_use]
pub fn kind_status(kind: std::io::ErrorKind) -> i32 {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound => NOT_FOUND,
        ErrorKind::PermissionDenied => PERMISSION_DENIED,
        ErrorKind::AlreadyExists => ALREADY_EXISTS,
        ErrorKind::InvalidInput | ErrorKind::InvalidData => INVALID_INPUT,
        ErrorKind::Interrupted => INTERRUPTED,
        ErrorKind::UnexpectedEof => UNEXPECTED_EOF,
        ErrorKind::BrokenPipe => BROKEN_PIPE,
        ErrorKind::TimedOut => TIMED_OUT,
        ErrorKind::Unsupported => UNSUPPORTED,
        _ => OTHER,
    }
}

/// Maps a `std::io::Error` to a stable status code.
#[must_use]
pub fn io_status(error: &std::io::Error) -> i32 {
    kind_status(error.kind())
}

/// Borrows a live managed string handle as UTF-8 text.
///
/// # Safety
/// A non-null handle must be live for the duration of the borrow.
pub unsafe fn text<'a>(value: *const ManagedString) -> Option<&'a str> {
    (!value.is_null()).then(|| unsafe { &*value }.as_str())
}

/// Borrows a live managed bytes handle.
///
/// # Safety
/// A non-null handle must be live for the duration of the borrow.
pub unsafe fn bytes<'a>(value: *const ManagedBytes) -> Option<&'a [u8]> {
    (!value.is_null()).then(|| unsafe { &*value }.as_slice())
}

/// Builds a NUL-separated list payload for `list(str)` results.
#[must_use]
pub fn join_fields(fields: &[String]) -> Vec<u8> {
    let mut payload = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            payload.push(0);
        }
        payload.extend_from_slice(field.as_bytes());
    }
    payload
}

/// Wraps UTF-8 text in a managed string, cloning from a borrowed slice.
#[must_use]
pub fn managed(value: &str) -> *mut ManagedString {
    managed_string(value)
}
