//! Process-wide Tokio runtime and pooled `reqwest` clients.
//!
//! These are implementation details: Vut source never observes Tokio or
//! reqwest. Sharing the runtime and client lets connection pools be reused.
use std::sync::OnceLock;

use reqwest::redirect::Policy;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static DEFAULT_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// The bounded redirect policy used when no explicit limit is requested.
#[must_use]
pub fn default_redirect_policy() -> Policy {
    Policy::limited(10)
}

/// Returns a client that follows a bounded number of redirects.
#[must_use]
pub fn build_client(policy: Policy) -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(policy)
        .build()
        .expect("vut native HTTP client")
}

/// Returns the shared default client.
///
/// # Panics
/// Panics only if the client cannot be created, which indicates a broken native
/// environment rather than a user error.
#[must_use]
pub fn default_client() -> &'static reqwest::Client {
    DEFAULT_CLIENT.get_or_init(|| build_client(default_redirect_policy()))
}

/// Returns the shared Tokio runtime, building it on first use.
///
/// The runtime is multi-threaded so that spawned request tasks make progress on
/// worker threads while the Vut thread polls the future without blocking.
///
/// # Panics
/// Panics only if the Tokio runtime cannot be created, which indicates a broken
/// native environment rather than a user error.
#[must_use]
pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("vut native HTTP runtime")
    })
}
