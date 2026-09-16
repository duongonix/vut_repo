//! Native async HTTP request operation.
//!
//! `vut_rt_http_send_v1` starts a request as a Tokio task and returns a generic
//! runtime future handle. The Vut `await` drives it through the shared async
//! ABI: the task stores the outcome and signals the handle; polling encodes the
//! response envelope. No HTTP-specific poll/waker/executor exists here, and the
//! Vut thread is never blocked on the Tokio future.
//!
//! Request parsing/execution lives in [`super::request`] and envelope encoding
//! in [`super::response`]; this module only owns the poll/drop seam.
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use vut_runtime::abi::{ManagedString, vut_rt_async_new_v1, vut_rt_async_signal_v1};
use vut_runtime::bytes::ManagedBytes;

use super::request::{build_spec, execute};
use super::response::{Outcome, encode_error, encode_ok};
use super::{client, error, runtime};
use crate::abi;

/// State shared between the operation, the polling callback, and the task.
struct Shared {
    handle: usize,
    cancelled: bool,
    outcome: Option<Outcome>,
}

/// Opaque operation object handed to the runtime future ABI.
pub struct SendOp {
    shared: Arc<Mutex<Shared>>,
}

/// Poll callback: encodes the completed outcome into `out`, or reports pending.
///
/// # Safety
/// `op` must be a live `SendOp` and `out` must have room for the result pointer.
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the async ABI out slot is aligned for the result type"
)]
unsafe extern "C" fn poll(op: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: the runtime passes back the `SendOp` created for this handle.
    let op = unsafe { &*(op as *const SendOp) };
    let shared = op
        .shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match &shared.outcome {
        Some(Outcome::Response {
            status,
            headers,
            body,
        }) => {
            let pointer = encode_ok(*status, headers, body);
            // SAFETY: `out` holds a result pointer for a `bytes` value.
            unsafe { *out.cast::<*mut ManagedBytes>() = pointer };
            1
        }
        Some(Outcome::Failure { kind, message }) => {
            let pointer = encode_error(*kind, message);
            // SAFETY: `out` holds a result pointer for a `bytes` value.
            unsafe { *out.cast::<*mut ManagedBytes>() = pointer };
            1
        }
        None => 0,
    }
}

/// Drop callback: cancels the operation and releases the operation object. The
/// task keeps the shared state alive and will not signal a dropped handle.
///
/// # Safety
/// `op` must be the pointer passed to `vut_rt_async_new_v1`.
unsafe extern "C" fn drop_op(op: *mut c_void) {
    // SAFETY: `op` was produced by `Box::into_raw` for a `SendOp`.
    let op = unsafe { Box::from_raw(op.cast::<SendOp>()) };
    let mut shared = op
        .shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    shared.cancelled = true;
}

fn ready_failure(kind: i32, message: &str) -> *mut c_void {
    let shared = Arc::new(Mutex::new(Shared {
        handle: 0,
        cancelled: false,
        outcome: Some(Outcome::Failure {
            kind,
            message: message.to_string(),
        }),
    }));
    let op = Box::into_raw(Box::new(SendOp { shared }));
    // SAFETY: `op` is a live operation object for the runtime.
    unsafe { vut_rt_async_new_v1(op.cast(), poll, drop_op).cast() }
}

/// Starts one HTTP request and returns a generic future handle for the response
/// envelope (`bytes`).
///
/// # Safety
/// Every pointer must be null or a live managed handle for the duration of the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_http_send_v1(
    client: *mut c_void,
    method: *const ManagedString,
    url: *const ManagedString,
    headers: *const ManagedString,
    query: *const ManagedString,
    body: *const ManagedBytes,
    timeout_nanos: usize,
    redirects: usize,
) -> *mut c_void {
    // SAFETY: the caller passes a live borrowed `Client` resource or null.
    let borrowed = if client.is_null() {
        None
    } else {
        Some(
            unsafe { &*client.cast::<client::ClientOp>() }
                .client
                .clone(),
        )
    };
    // SAFETY: the caller passes live managed handles or null.
    let (Some(method), Some(url)) = (unsafe { abi::text(method) }, unsafe { abi::text(url) })
    else {
        return ready_failure(error::INVALID_URL, "invalid method or url");
    };
    // SAFETY: the caller passes live managed handles or null.
    let headers = unsafe { abi::text(headers) }.unwrap_or("");
    let query = unsafe { abi::text(query) }.unwrap_or("");
    let body = unsafe { abi::bytes(body) }.unwrap_or(&[]);

    let spec = match build_spec(method, url, headers, query, body, timeout_nanos, redirects) {
        Ok(spec) => spec,
        Err((kind, message)) => return ready_failure(kind, &message),
    };

    let shared = Arc::new(Mutex::new(Shared {
        handle: 0,
        cancelled: false,
        outcome: None,
    }));
    let op = Box::into_raw(Box::new(SendOp {
        shared: Arc::clone(&shared),
    }));
    // SAFETY: `op` is a live operation object for the runtime.
    let handle = unsafe { vut_rt_async_new_v1(op.cast(), poll, drop_op) };
    let handle_bits = handle as usize;
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .handle = handle_bits;

    runtime::runtime().spawn(async move {
        let http = borrowed.unwrap_or_else(|| client::client_for(spec.redirects, 0));
        let outcome = execute(spec, http).await;
        let mut state = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.outcome = Some(outcome);
        if !state.cancelled {
            // Hold the lock while signaling so a concurrent drop cannot free
            // the handle before the wake is delivered.
            let handle =
                std::ptr::with_exposed_provenance_mut::<vut_runtime::AsyncHandle>(handle_bits);
            // SAFETY: the handle is live and not cancelled.
            unsafe { vut_rt_async_signal_v1(handle) };
        }
    });

    handle.cast::<c_void>()
}
