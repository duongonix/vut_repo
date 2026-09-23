//! Pooled `reqwest` clients and their native resource handles.
//!
//! A shared default client (bounded redirects) is used when no explicit limit is
//! requested; other (redirects, timeout) combinations get a cached client so
//! connection pools are reused. `vut_rt_http_client_create_v1` returns a resource
//! owning a cached client; the resource is borrowed (not moved) by requests.
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::time::Duration;

use reqwest::redirect::Policy;
use vut_runtime::abi::vut_rt_resource_new_v1;

use super::runtime;

static CLIENTS: Mutex<Option<HashMap<(usize, usize), reqwest::Client>>> = Mutex::new(None);

/// Builds a client with a redirect policy and an optional default timeout.
///
/// `redirects == 0` selects the backend's bounded default policy so that a
/// client configured only with a timeout still follows redirects like the
/// default client. A positive value sets an explicit limit.
fn build(redirects: usize, timeout_nanos: usize) -> reqwest::Client {
    let policy = if redirects == 0 {
        runtime::default_redirect_policy()
    } else {
        Policy::limited(redirects)
    };
    let mut builder = reqwest::Client::builder().redirect(policy);
    if timeout_nanos > 0 {
        builder = builder.timeout(Duration::from_nanos(
            u64::try_from(timeout_nanos).unwrap_or(u64::MAX),
        ));
    }
    crate::guard::or_abort(builder.build(), "http client")
}

/// Returns a pooled client honoring the requested redirect and timeout policy.
///
/// `redirects == 0` selects the shared default client when no timeout is set,
/// and otherwise a cached client with the default redirect policy.
#[must_use]
pub fn client_for(redirects: usize, timeout_nanos: usize) -> reqwest::Client {
    if redirects == 0 && timeout_nanos == 0 {
        return runtime::default_client().clone();
    }
    let mut cache = CLIENTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let cache = cache.get_or_insert_with(HashMap::new);
    cache
        .entry((redirects, timeout_nanos))
        .or_insert_with(|| build(redirects, timeout_nanos))
        .clone()
}

/// Native `Client` state owned by a `resource(HttpClient)`.
pub struct ClientOp {
    pub client: reqwest::Client,
}

/// # Safety
/// `op` must be the pointer passed to `vut_rt_resource_new_v1`.
unsafe extern "C" fn drop_client(op: *mut c_void) {
    // SAFETY: `op` was produced by `Box::into_raw` for a `ClientOp`.
    let _ = unsafe { Box::from_raw(op.cast::<ClientOp>()) };
}

/// Creates a `Client` resource owning a pooled reqwest client.
///
/// # Safety
/// Always safe to call; the returned handle must be released by the runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_http_client_create_v1(
    timeout_nanos: usize,
    redirects: usize,
) -> *mut c_void {
    let op = Box::into_raw(Box::new(ClientOp {
        client: client_for(redirects, timeout_nanos),
    }));
    // SAFETY: `op` is a live operation object for the runtime.
    unsafe { vut_rt_resource_new_v1(op.cast(), Some(drop_client)) }
}
