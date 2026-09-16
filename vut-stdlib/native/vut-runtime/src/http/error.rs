//! Native HTTP transport error codes and reqwest error classification.
//!
//! These codes are carried in the response envelope and mapped to `http.Error`
//! kinds by the Vut layer.
pub const OK_KIND: i32 = 0;
pub const INVALID_URL: i32 = 1;
pub const DNS: i32 = 2;
pub const CONNECT: i32 = 3;
pub const TLS: i32 = 4;
pub const TIMEOUT: i32 = 5;
pub const REQUEST: i32 = 6;
pub const REDIRECT: i32 = 7;
pub const RESPONSE: i32 = 8;
pub const BODY: i32 = 9;
pub const UNSUPPORTED: i32 = 11;
pub const OTHER: i32 = 99;

/// Classifies a `reqwest` transport error into a native error code.
#[must_use]
pub fn kind(error: &reqwest::Error) -> i32 {
    if error.is_timeout() {
        return TIMEOUT;
    }
    if error.is_builder() {
        return INVALID_URL;
    }
    if error.is_redirect() {
        return REDIRECT;
    }
    if error.is_body() {
        return BODY;
    }
    if error.is_decode() {
        return RESPONSE;
    }
    if error.is_request() {
        let text = error.to_string();
        if text.contains("dns") || text.contains("name resolution") || text.contains("resolve") {
            return DNS;
        }
        if text.contains("tls") || text.contains("certificate") || text.contains("handshake") {
            return TLS;
        }
        if text.contains("connect") {
            return CONNECT;
        }
        return REQUEST;
    }
    OTHER
}
