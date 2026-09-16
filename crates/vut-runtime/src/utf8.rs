//! UTF-8 validation shared by the byte-buffer runtime.
//!
//! Validation always operates on the raw contiguous bytes of a `bytes`
//! buffer. No lossy replacement or truncation is performed; callers receive
//! the exact failure location so a typed `Utf8Error` can be produced.

/// Location of the first invalid UTF-8 sequence in a byte slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Utf8Issue {
    /// Number of leading bytes that form valid UTF-8.
    pub valid_up_to: usize,
    /// Length of the invalid sequence, or `None` when the input ended
    /// prematurely (an incomplete trailing sequence).
    pub error_len: Option<usize>,
}

/// Classifies a byte slice as valid UTF-8 or reports the first failure.
///
/// # Errors
/// Returns [`Utf8Issue`] describing the first invalid or incomplete sequence.
pub fn validate(bytes: &[u8]) -> Result<(), Utf8Issue> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|error| Utf8Issue {
            valid_up_to: error.valid_up_to(),
            error_len: error.error_len(),
        })
}
