//! Native HTTP client primitives (`vut_rt_http_*`).
//!
//! Built on `reqwest` + `tokio` + `rustls`. Only the stable C ABI surface
//! declared here crosses into Vut; no Rust type from those crates is exposed.
//! High-level behavior lives in `vut-stdlib/std/http`.
//!
//! The request operation is a generic runtime future (see `future.rs`). There is
//! no HTTP-specific executor, poller, or blocking wait: the Vut `await` drives
//! the shared async ABI.
mod client;
mod error;
mod future;
mod request;
mod response;
mod runtime;
mod stream;

pub use client::vut_rt_http_client_create;
pub use future::vut_rt_http_send_v1;
pub use stream::{
    vut_rt_http_stream_chunk_v1, vut_rt_http_stream_read_v1, vut_rt_http_stream_send_v1,
};

/// Link/ABI probe used to validate the native HTTP backend is reachable.
#[unsafe(no_mangle)]
pub extern "C" fn vut_rt_http_probe_v1() -> usize {
    let _client = reqwest::Client::new();
    let _runtime = runtime::runtime();
    1
}
