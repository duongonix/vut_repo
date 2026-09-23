//! Vut benchmark and measurement infrastructure (M2.6.1).
//!
//! Compiles the benchmark corpus at O0–O3, measures wall time, estimated CPU
//! time, peak RSS, compile time, binary size, and deterministic MIR structure
//! metrics, verifies each workload's output, and persists a JSON baseline that
//! later M2.6 phases can compare against.
//!
//! Optimization level and CPU target are independent: the CPU target defaults
//! to the portable host triple and native codegen is never enabled implicitly.
#![allow(clippy::missing_errors_doc)]

pub mod competitors;
pub mod corpus;
pub mod metrics;
pub mod model;
pub mod process;
pub mod report;
pub mod runner;
pub mod structure;

pub use model::{BenchReport, Level, RunStats, StructureMetrics, WorkloadResult};
pub use runner::Runner;
