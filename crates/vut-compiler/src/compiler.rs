//! Compiler-session foundation. Language stages are added in later phases.
use std::path::PathBuf;
use vut_diagnostics::DiagnosticSink;
use vut_hir::HirProgram;
use vut_mir::Program as MirProgram;
use vut_resolver::Resolution;
use vut_source::SourceManager;
use vut_types::SemanticResult;

mod display;
mod error;
mod permissions;
mod session;
#[cfg(test)]
mod tests;

pub use error::CompileError;
pub use vut_mir::OptimizationLevel;

/// Runtime ABI revision the compiler targets. The installed distribution's
/// `manifest.json:abi_version` must match this.
pub const RUNTIME_ABI_VERSION: u32 = vut_runtime::abi::VERSION;

/// Returns a [`CompileError::AbiMismatch`] when an installed runtime ABI does
/// not match the compiler's. `None` (no install manifest) is always compatible.
#[must_use]
pub fn runtime_abi_mismatch(installed: Option<u32>) -> Option<CompileError> {
    installed
        .filter(|installed| *installed != RUNTIME_ABI_VERSION)
        .map(|runtime| CompileError::AbiMismatch {
            compiler: RUNTIME_ABI_VERSION,
            runtime,
        })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BuildMode {
    #[default]
    Debug,
    Release,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilerConfig {
    pub target: String,
    /// Target CPU: `None` is the portable baseline; `Some("native")` or a
    /// preset opts into that CPU's features. Independent of the optimization
    /// level.
    pub target_cpu: Option<String>,
    /// Explicit Cranelift feature overrides (`avx2`, `+avx2`, `-avx2`).
    pub target_features: Vec<String>,
    pub build_mode: BuildMode,
    /// Explicit MIR/codegen optimization level. When `None` it is derived from
    /// `build_mode`: Debug uses the reference path (`O0`), Release uses `O2`.
    pub optimization: Option<vut_mir::OptimizationLevel>,
    pub output_dir: Option<PathBuf>,
    pub runtime_library: Option<PathBuf>,
    /// Prebuilt `vut-startup` object (`vut-startup.o`/`.obj`) for standalone
    /// linking. The rustc backend supplies its own entry shim and ignores it.
    pub startup_object: Option<PathBuf>,
    /// Native static libraries (`.lib`/`.a`) to link into the executable.
    pub native_libraries: Vec<PathBuf>,
    /// Platform system libraries referenced by native dependencies.
    pub system_libraries: Vec<String>,
    /// Extra library search directories (`/LIBPATH:` on MSVC, `-L` elsewhere).
    pub library_search_paths: Vec<PathBuf>,
}
impl CompilerConfig {
    #[must_use]
    pub fn for_target(target: impl Into<String>, build_mode: BuildMode) -> Self {
        Self {
            target: target.into(),
            target_cpu: None,
            target_features: Vec::new(),
            build_mode,
            optimization: None,
            output_dir: None,
            runtime_library: None,
            startup_object: None,
            native_libraries: Vec::new(),
            system_libraries: Vec::new(),
            library_search_paths: Vec::new(),
        }
    }
}
impl CompilerConfig {
    /// The codegen target, including the opt-in CPU/feature configuration.
    #[must_use]
    pub fn codegen_target(&self) -> vut_codegen::Target {
        vut_codegen::Target {
            triple: self.target.clone(),
            cpu: self.target_cpu.clone(),
            features: self.target_features.clone(),
        }
    }
}
impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: vut_codegen::Target::host().triple,
            target_cpu: None,
            target_features: Vec::new(),
            build_mode: BuildMode::Debug,
            optimization: None,
            output_dir: None,
            runtime_library: None,
            startup_object: None,
            native_libraries: Vec::new(),
            system_libraries: Vec::new(),
            library_search_paths: Vec::new(),
        }
    }
}
#[derive(Debug)]
pub struct CompilerSession {
    config: CompilerConfig,
    sources: SourceManager,
    diagnostics: DiagnosticSink,
}
#[derive(Debug)]
pub struct CheckedProgram {
    pub resolution: Resolution,
    pub hir: HirProgram,
    pub semantics: SemanticResult,
    pub mir: MirProgram,
}
