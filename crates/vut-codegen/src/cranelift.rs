//! Cranelift implementation of the Vut native backend.
use std::fmt;
use target_lexicon::Triple;
use vut_mir::Program;

pub const RUNTIME_ABI_VERSION: u32 = 13;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    pub triple: String,
}
impl Target {
    #[must_use]
    pub fn host() -> Self {
        Self {
            triple: target_lexicon::HOST.to_string(),
        }
    }
}

/// Returns the target pointer width in bytes for a target triple, falling
/// back to the host pointer width when the triple is unknown.
#[must_use]
pub fn pointer_bytes_for_target(triple: &str) -> usize {
    triple
        .parse::<Triple>()
        .ok()
        .and_then(|triple| triple.pointer_width().ok())
        .map_or(std::mem::size_of::<usize>(), |width| match width {
            target_lexicon::PointerWidth::U16 => 2,
            target_lexicon::PointerWidth::U32 => 4,
            target_lexicon::PointerWidth::U64 => 8,
        })
}
#[derive(Debug)]
pub enum CodegenError {
    InvalidTarget(String),
    Backend(String),
    RuntimeAbi { compiler: u32, runtime: u32 },
}
impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget(e) | Self::Backend(e) => f.write_str(e),
            Self::RuntimeAbi { compiler, runtime } => write!(
                f,
                "runtime ABI mismatch: compiler {compiler}, runtime {runtime}"
            ),
        }
    }
}
impl std::error::Error for CodegenError {}
pub trait CodegenBackend {
    fn target(&self) -> &Target;
    /// Emits one target-native object.
    ///
    /// # Errors
    /// Returns an error when MIR violates an invariant or the backend rejects it.
    fn compile_module(&self, program: &Program) -> Result<Vec<u8>, CodegenError>;
}
pub struct CraneliftBackend {
    target: Target,
    optimize: bool,
}
impl CraneliftBackend {
    #[must_use]
    pub fn new(target: Target) -> Self {
        Self {
            target,
            optimize: false,
        }
    }
    #[must_use]
    pub fn with_optimization(target: Target, optimize: bool) -> Self {
        Self { target, optimize }
    }

    /// Emits an object with a platform entry shim. A `void` Vut entry returns
    /// process exit code zero; an integer entry forwards its value.
    ///
    /// # Errors
    /// Returns an error when MIR violates an invariant or the backend rejects it.
    pub fn compile_executable_module(
        &self,
        program: &Program,
        entry: vut_mir::SymbolId,
    ) -> Result<Vec<u8>, CodegenError> {
        self.compile(program, Some((entry, false)))
    }

    /// Emits an executable and optionally makes the entry shim fail with exit
    /// code 254 when runtime-managed strings remain live after `main`.
    ///
    /// # Errors
    ///
    /// Returns an error when MIR lowering, target code generation, or object
    /// emission fails.
    pub fn compile_executable_module_with_memory_check(
        &self,
        program: &Program,
        entry: vut_mir::SymbolId,
        memory_check: bool,
    ) -> Result<Vec<u8>, CodegenError> {
        self.compile(program, Some((entry, memory_check)))
    }

    fn compile(
        &self,
        program: &Program,
        entry: Option<(vut_mir::SymbolId, bool)>,
    ) -> Result<Vec<u8>, CodegenError> {
        compile(self, program, entry)
    }
}
impl CodegenBackend for CraneliftBackend {
    fn target(&self) -> &Target {
        &self.target
    }
    fn compile_module(&self, program: &Program) -> Result<Vec<u8>, CodegenError> {
        self.compile(program, None)
    }
}

mod compile;
mod futures;
mod instruction;
mod managed;
mod signatures;
#[cfg(test)]
mod tests;

use compile::compile;

/// Verifies compiler/runtime internal ABI compatibility.
///
/// # Errors
/// Returns a version mismatch when the runtime uses another ABI revision.
pub fn verify_runtime_abi(runtime: u32) -> Result<(), CodegenError> {
    if runtime == RUNTIME_ABI_VERSION {
        Ok(())
    } else {
        Err(CodegenError::RuntimeAbi {
            compiler: RUNTIME_ABI_VERSION,
            runtime,
        })
    }
}
