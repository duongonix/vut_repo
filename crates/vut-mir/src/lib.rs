//! Backend-independent ownership-aware MIR facade.
pub mod analyze;
mod lowering;
mod optimize;
pub use lowering::*;
pub use optimize::{
    EscapeSummary, OptimizationLevel, OptimizationReport, VerificationError, escape_analysis,
    optimize, verify,
};
pub use vut_diagnostics::DiagnosticSink;
