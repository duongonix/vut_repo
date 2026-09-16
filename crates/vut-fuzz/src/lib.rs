//! Deterministic long-running frontend corpus fuzz harness.
mod generator;
mod runner;

pub use runner::{FuzzReport, run};
