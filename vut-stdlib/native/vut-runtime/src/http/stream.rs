//! Native streaming HTTP response body.
//!
//! `vut_rt_http_stream_send_v1` starts a request on a Tokio worker and returns a
//! `resource` wrapping a bounded channel (capacity 1) of body chunks.
//! `vut_rt_http_stream_read_v1` is a generic runtime future that awaits the next
//! chunk. Memory stays bounded (one in-flight chunk); there is no HTTP-specific
//! executor and the Vut thread is never blocked on Tokio.
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use vut_runtime::abi::{
    ManagedString, vut_rt_async_new_v1, vut_rt_async_signal_v1, vut_rt_resource_new_v1,
};
use vut_runtime::bytes::{ManagedBytes, managed_bytes};

use super::request::build_spec;
use super::{client, error, runtime};
use crate::abi;

/// A chunk read from the body, a transport error, or end of stream.
type Chunk = Result<Vec<u8>, (i32, String)>;

const CHUNK_KIND: i32 = 0;
const EOF_KIND: i32 = 1;

/// Native stream state owned by a `resource(HttpStream)`.
pub struct StreamOp {
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Chunk>>>,
}

/// Outcome of one read future.
enum ReadOutcome {
    Chunk(Vec<u8>),
    Eof,
    Error(i32, String),
}

struct ReadShared {
    handle: usize,
    cancelled: bool,
    outcome: Option<ReadOutcome>,
}

struct ReadOp {
    shared: Arc<Mutex<ReadShared>>,
}

fn encode(outcome: &ReadOutcome) -> *mut ManagedBytes {
    let mut buffer = Vec::new();
    match outcome {
        ReadOutcome::Chunk(bytes) => {
            buffer.extend_from_slice(&CHUNK_KIND.to_le_bytes());
            buffer.extend_from_slice(bytes);
        }
        ReadOutcome::Eof => buffer.extend_from_slice(&EOF_KIND.to_le_bytes()),
        ReadOutcome::Error(kind, message) => {
            buffer.extend_from_slice(&kind.to_le_bytes());
            buffer.extend_from_slice(message.as_bytes());
        }
    }
    managed_bytes(buffer)
}

/// # Safety
/// `op` must be the pointer passed to `vut_rt_resource_new_v1`.
unsafe extern "C" fn drop_stream(op: *mut c_void) {
    // SAFETY: `op` was produced by `Box::into_raw` for a `StreamOp`.
    let _ = unsafe { Box::from_raw(op.cast::<StreamOp>()) };
}

/// Poll callback for a read future.
///
/// # Safety
/// `op` must be a live `ReadOp` and `out` must have room for the result pointer.
#[expect(
    clippy::cast_ptr_alignment,
    reason = "the async ABI out slot is aligned for the result type"
)]
unsafe extern "C" fn poll_read(op: *mut c_void, out: *mut u8) -> i32 {
    // SAFETY: the runtime passes back the `ReadOp` created for this handle.
    let op = unsafe { &*(op as *const ReadOp) };
    let shared = op
        .shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match &shared.outcome {
        Some(outcome) => {
            let pointer = encode(outcome);
            // SAFETY: `out` holds a result pointer for a `bytes` value.
            unsafe { *out.cast::<*mut ManagedBytes>() = pointer };
            1
        }
        None => 0,
    }
}

/// Drop callback for a read future.
///
/// # Safety
/// `op` must be the pointer passed to `vut_rt_async_new_v1`.
unsafe extern "C" fn drop_read(op: *mut c_void) {
    // SAFETY: `op` was produced by `Box::into_raw` for a `ReadOp`.
    let op = unsafe { Box::from_raw(op.cast::<ReadOp>()) };
    let mut shared = op
        .shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    shared.cancelled = true;
}

/// Starts a streaming request and returns a stream resource.
///
/// # Safety
/// Every pointer must be null or a live managed handle for the duration of the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_http_stream_send_v1(
    client: *mut c_void,
    method: *const ManagedString,
    url: *const ManagedString,
    headers: *const ManagedString,
    query: *const ManagedString,
    body: *const ManagedBytes,
    timeout_nanos: usize,
    redirects: usize,
) -> *mut c_void {
    let (sender, receiver) = mpsc::channel::<Chunk>(1);

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
    let (method, url) = (unsafe { abi::text(method) }, unsafe { abi::text(url) });
    // SAFETY: the caller passes live managed handles or null.
    let headers = unsafe { abi::text(headers) }.unwrap_or("");
    let query = unsafe { abi::text(query) }.unwrap_or("");
    let body = unsafe { abi::bytes(body) }.unwrap_or(&[]);

    let spec = match (method, url) {
        (Some(method), Some(url)) => {
            build_spec(method, url, headers, query, body, timeout_nanos, redirects)
        }
        _ => Err((error::INVALID_URL, "invalid method or url".to_owned())),
    };

    match spec {
        Ok(spec) => {
            runtime::runtime().spawn(async move {
                let http = borrowed.unwrap_or_else(|| client::client_for(spec.redirects, 0));
                let mut request = http
                    .request(spec.method, spec.url)
                    .headers(spec.headers)
                    .body(spec.body);
                if let Some(timeout) = spec.timeout {
                    request = request.timeout(timeout);
                }
                match request.send().await {
                    Ok(mut response) => loop {
                        match response.chunk().await {
                            Ok(Some(chunk)) => {
                                if sender.send(Ok(chunk.to_vec())).await.is_err() {
                                    return;
                                }
                            }
                            Ok(None) => return,
                            Err(error) => {
                                let _ = sender.send(Err((error::BODY, error.to_string()))).await;
                                return;
                            }
                        }
                    },
                    Err(error) => {
                        let _ = sender
                            .send(Err((error::kind(&error), error.to_string())))
                            .await;
                    }
                }
            });
        }
        Err((kind, message)) => {
            let _ = sender.try_send(Err((kind, message)));
        }
    }

    let op = Box::into_raw(Box::new(StreamOp {
        receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
    }));
    // SAFETY: `op` is a live operation object for the runtime.
    unsafe { vut_rt_resource_new_v1(op.cast(), Some(drop_stream)) }
}

/// Wraps an already-received body as a one-chunk stream, so `Response.stream()`
/// can hand back a `Stream` over a buffered response. Bounded-memory streaming
/// for large bodies uses `vut_rt_http_stream_send_v1` instead.
///
/// # Safety
/// `body` must be null or a live managed `bytes` handle for the duration of the
/// call; the bytes are copied before it is released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_http_stream_chunk_v1(body: *const ManagedBytes) -> *mut c_void {
    let (sender, receiver) = mpsc::channel::<Chunk>(1);
    // SAFETY: the caller passes a live managed handle or null.
    let data = unsafe { abi::bytes(body) }.unwrap_or(&[]).to_vec();
    // An empty body has no chunk: dropping the sender reports end of stream.
    runtime::runtime().spawn(async move {
        if !data.is_empty() {
            let _ = sender.send(Ok(data)).await;
        }
    });
    let op = Box::into_raw(Box::new(StreamOp {
        receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
    }));
    // SAFETY: `op` is a live operation object for the runtime.
    unsafe { vut_rt_resource_new_v1(op.cast(), Some(drop_stream)) }
}

/// Awaits the next body chunk from a stream resource, yielding a result
/// envelope (`0` chunk, `1` end of stream, otherwise a transport error code).
///
/// # Safety
/// `stream` must be null or a live stream resource (borrowed, not consumed).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vut_rt_http_stream_read_v1(stream: *mut c_void) -> *mut c_void {
    let receiver = if stream.is_null() {
        None
    } else {
        // SAFETY: the caller passes a live stream resource it still owns.
        Some(unsafe { &*(stream as *const StreamOp) }.receiver.clone())
    };

    let shared = Arc::new(Mutex::new(ReadShared {
        handle: 0,
        cancelled: false,
        outcome: None,
    }));
    let op = Box::into_raw(Box::new(ReadOp {
        shared: Arc::clone(&shared),
    }));
    // SAFETY: `op` is a live operation object for the runtime.
    let handle = unsafe { vut_rt_async_new_v1(op.cast(), poll_read, drop_read) };
    let handle_bits = handle as usize;
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .handle = handle_bits;

    match receiver {
        Some(receiver) => {
            runtime::runtime().spawn(async move {
                let queued = receiver.lock().await.recv().await;
                let outcome = match queued {
                    Some(Ok(bytes)) => ReadOutcome::Chunk(bytes),
                    Some(Err((kind, message))) => ReadOutcome::Error(kind, message),
                    None => ReadOutcome::Eof,
                };
                let mut state = shared
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                state.outcome = Some(outcome);
                if !state.cancelled {
                    // Hold the lock while signaling so a concurrent drop cannot
                    // free the handle before the wake is delivered.
                    let handle = std::ptr::with_exposed_provenance_mut::<vut_runtime::AsyncHandle>(
                        handle_bits,
                    );
                    // SAFETY: the handle is live and not cancelled.
                    unsafe { vut_rt_async_signal_v1(handle) };
                }
            });
        }
        None => {
            shared
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .outcome = Some(ReadOutcome::Error(
                error::INVALID_URL,
                "invalid stream".to_owned(),
            ));
        }
    }

    handle.cast::<c_void>()
}
