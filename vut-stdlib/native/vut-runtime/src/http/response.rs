//! HTTP response envelope encoding for the Vut ABI.
//!
//! The request side produces a transport-neutral [`Outcome`]; this module
//! serializes it into the `bytes` envelope the Vut layer decodes:
//!
//! ```text
//! success: kind(0) status(4) headers_len(4) headers body
//! failure: kind(4) message
//! ```
use reqwest::header::HeaderMap;
use vut_runtime::bytes::{ManagedBytes, managed_bytes};

use super::error;

/// Result of a completed request, holding no Vut handles so it can be produced
/// on a Tokio worker thread and encoded later on the Vut thread.
pub(super) enum Outcome {
    Response {
        status: u16,
        headers: HeaderMap,
        body: Vec<u8>,
    },
    Failure {
        kind: i32,
        message: String,
    },
}

/// Renders headers into the flat `Name: value` block the Vut side parses.
pub(super) fn render_headers(headers: &HeaderMap) -> String {
    let mut out = String::new();
    for (name, value) in headers {
        let Ok(text) = value.to_str() else { continue };
        out.push_str(name.as_str());
        out.push_str(": ");
        out.push_str(text);
        out.push('\n');
    }
    out
}

pub(super) fn encode_ok(status: u16, headers: &HeaderMap, body: &[u8]) -> *mut ManagedBytes {
    let headers = render_headers(headers);
    let mut out = Vec::with_capacity(12 + headers.len() + body.len());
    out.extend_from_slice(&error::OK_KIND.to_le_bytes());
    out.extend_from_slice(&i32::from(status).to_le_bytes());
    out.extend_from_slice(
        &i32::try_from(headers.len())
            .unwrap_or(i32::MAX)
            .to_le_bytes(),
    );
    out.extend_from_slice(headers.as_bytes());
    out.extend_from_slice(body);
    managed_bytes(out)
}

pub(super) fn encode_error(kind: i32, message: &str) -> *mut ManagedBytes {
    let mut out = Vec::with_capacity(4 + message.len());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(message.as_bytes());
    managed_bytes(out)
}
