//! Compiler orchestration error type.
use std::fmt;

use vut_resolver::DiscoverError;

#[derive(Debug)]
pub enum CompileError {
    Discover(DiscoverError),
    Language(String),
    Codegen(vut_codegen::CodegenError),
    Link(vut_linker::LinkError),
    Io(std::io::Error),
    MissingEntry,
    InvalidEntry,
    Cache(vut_incremental::CacheError),
    /// The installed runtime's ABI revision does not match the compiler's.
    AbiMismatch {
        compiler: u32,
        runtime: u32,
    },
}
impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discover(error) => write!(formatter, "source discovery failed: {error}"),
            Self::Language(diagnostics) => {
                write!(formatter, "compilation failed\n{diagnostics}")
            }
            Self::Codegen(error) => write!(formatter, "native code generation failed: {error}"),
            Self::Link(error) => write!(formatter, "native linking failed: {error}"),
            Self::Io(error) => write!(formatter, "artifact I/O failed: {error}"),
            Self::MissingEntry => formatter.write_str("program has no `fn main(...)` entry point"),
            Self::InvalidEntry => {
                formatter.write_str("`main` must have no parameters and return `void` or `int`")
            }
            Self::Cache(error) => write!(formatter, "incremental cache failed: {error}"),
            Self::AbiMismatch { compiler, runtime } => write!(
                formatter,
                "runtime ABI mismatch: compiler expects {compiler}, installed runtime is {runtime}; \
                 reinstall or update Vut so the compiler and runtime match"
            ),
        }
    }
}
impl std::error::Error for CompileError {}
