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
        }
    }
}
impl std::error::Error for CompileError {}
