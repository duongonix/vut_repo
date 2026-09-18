//! Shared panic and bounds-failure path.
//!
//! There is exactly one bounds-failure entry point. Generated code and the
//! runtime collection operations both funnel invalid indexes through it so the
//! failure is deterministic and carries operation/index/length context.
use std::io::Write;

/// Operation identities for [`vut_rt_bounds_panic_v1`]. The values are part of
/// the runtime ABI and must stay stable.
pub mod bounds_op {
    pub const LIST_AT: i64 = 0;
    pub const LIST_SET: i64 = 1;
    pub const LIST_INSERT: i64 = 2;
    pub const LIST_REMOVE: i64 = 3;
    pub const BYTES_AT: i64 = 4;
    pub const BYTES_SET: i64 = 5;
    pub const ARRAY_AT: i64 = 6;
    pub const ARRAY_SET: i64 = 7;
    pub const ARRAY_FIRST: i64 = 8;
    pub const ARRAY_LAST: i64 = 9;

    /// Human-readable name for an operation id.
    #[must_use]
    pub fn name(op: i64) -> &'static str {
        match op {
            LIST_AT => "list.at",
            LIST_SET => "list.set",
            LIST_INSERT => "list.insert",
            LIST_REMOVE => "list.remove",
            BYTES_AT => "bytes.at",
            BYTES_SET => "bytes.set",
            ARRAY_AT => "array.at",
            ARRAY_SET => "array.set",
            ARRAY_FIRST => "array.first",
            ARRAY_LAST => "array.last",
            _ => "<index>",
        }
    }
}

/// Recovers the original signed index from a `usize` argument. Vut `int`
/// indexes are 64-bit signed and a negative value arrives as its
/// two's-complement `usize` bit pattern; this restores it for the message.
#[must_use]
pub fn signed_index(index: usize) -> i64 {
    i64::from_ne_bytes(index.to_ne_bytes())
}

/// Writes `message` to stderr and aborts the process.
///
/// # Safety
/// A non-null `message` must point to `length` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_panic_v1(message: *const u8, length: usize) -> ! {
    let bytes = if message.is_null() {
        &[][..]
    } else {
        // SAFETY: upheld by the caller.
        unsafe { std::slice::from_raw_parts(message, length) }
    };
    let text = String::from_utf8_lossy(bytes);
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(lock, "vut panic: {text}");
    let _ = lock.flush();
    std::process::abort()
}

/// Traps with a formatted bounds message. Shared by generated code and the
/// runtime collection operations.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_bounds_panic_v1(op: i64, index: i64, length: i64) -> ! {
    let message = format!(
        "{}: index {index} out of range (length {length})",
        bounds_op::name(op)
    );
    // SAFETY: `message` is a live `String`; the pointer and length are valid for
    // the duration of the call, which never returns.
    unsafe { vut_rt_panic_v1(message.as_ptr(), message.len()) }
}

/// Returns `index` when it is a valid index for `length`, otherwise traps.
/// Generated code uses this for inline (array) accesses.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_bounds_check_v1(op: i64, index: i64, length: i64) -> i64 {
    if index < 0 || index >= length {
        vut_rt_bounds_panic_v1(op, index, length);
    }
    index
}
