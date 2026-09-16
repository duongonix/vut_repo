//! Backend-independent ownership-aware MIR facade.
mod lowering;
mod optimize;
pub use lowering::*;
pub use optimize::{OptimizationReport, optimize};
pub use vut_diagnostics::DiagnosticSink;
