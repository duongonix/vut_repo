//! `vut-dist` command line.
use std::path::PathBuf;

use clap::Parser;

/// Assembles a Vut distribution artifact for a target.
#[derive(Debug, Parser)]
#[command(name = "vut-dist", version, about)]
pub struct Args {
    /// Target triple, e.g. `x86_64-pc-windows-msvc`.
    #[arg(long)]
    pub target: String,
    /// Output directory for the staging tree, archive and SHA256SUMS.
    #[arg(long)]
    pub out: PathBuf,
    /// Directory containing the built `vut`, `vpm`, `vut-lsp` and runtime archives.
    #[arg(long, default_value = "target/release")]
    pub bin_dir: PathBuf,
    /// Standard library `.vut` source root (defaults to the bundled tree).
    #[arg(long)]
    pub std: Option<PathBuf>,
    /// Prebuilt startup object. Built with `rustc` when omitted.
    #[arg(long)]
    pub startup: Option<PathBuf>,
    /// VPM version for the manifest (defaults to the distribution version).
    #[arg(long)]
    pub vpm_version: Option<String>,
    /// Source revision for the manifest (defaults to `GITHUB_SHA`).
    #[arg(long)]
    pub build_commit: Option<String>,
    /// Minimum supported OS for the manifest (optional).
    #[arg(long)]
    pub minimum_os: Option<String>,
}
