//! Backend-neutral native object emission API.
mod cranelift;
pub use cranelift::{
    CodegenBackend, CodegenError, CraneliftBackend, RUNTIME_ABI_VERSION, Target,
    pointer_bytes_for_target, verify_runtime_abi,
};
