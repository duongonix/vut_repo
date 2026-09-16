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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BuildMode {
    #[default]
    Debug,
    Release,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilerConfig {
    pub target: String,
    pub build_mode: BuildMode,
    pub output_dir: Option<PathBuf>,
    pub runtime_library: Option<PathBuf>,
    /// Native static libraries (`.lib`/`.a`) to link into the executable.
    pub native_libraries: Vec<PathBuf>,
    /// Platform system libraries referenced by native dependencies.
    pub system_libraries: Vec<String>,
}
impl CompilerConfig {
    #[must_use]
    pub fn for_target(target: impl Into<String>, build_mode: BuildMode) -> Self {
        Self {
            target: target.into(),
            build_mode,
            output_dir: None,
            runtime_library: None,
            native_libraries: Vec::new(),
            system_libraries: Vec::new(),
        }
    }
}
impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: vut_codegen::Target::host().triple,
            build_mode: BuildMode::Debug,
            output_dir: None,
            runtime_library: None,
            native_libraries: Vec::new(),
            system_libraries: Vec::new(),
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
