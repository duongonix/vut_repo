//! `vut-dist` command line.
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Vut distribution packaging tool.
#[derive(Debug, Parser)]
#[command(name = "vut-dist", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Assemble one target distribution (staging tree, archive, manifest, checksum).
    Assemble(AssembleArgs),
    /// Merge per-target manifests into a release-level `releases.json`.
    ReleaseManifest(ReleaseArgs),
}

/// Arguments for `assemble`.
#[derive(Debug, Args)]
pub struct AssembleArgs {
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
    /// Cranelift version recorded in the manifest/version metadata.
    #[arg(long)]
    pub cranelift: Option<String>,
    /// Release channel recorded in the manifest/version metadata.
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// License file to ship in the artifact (defaults to the repository `LICENSE`).
    #[arg(long)]
    pub license: Option<PathBuf>,
    /// Third-party notices to ship when a redistributable is bundled.
    #[arg(long)]
    pub notices: Option<PathBuf>,
}

/// Arguments for `release-manifest`.
#[derive(Debug, Args)]
pub struct ReleaseArgs {
    /// Directory containing the per-target archives and `<dist>.manifest.json` files.
    #[arg(long)]
    pub dir: PathBuf,
    /// Output path for `releases.json`.
    #[arg(long)]
    pub out: PathBuf,
    /// Release channel recorded in the manifest.
    #[arg(long, default_value = "stable")]
    pub channel: String,
    /// Cranelift version recorded in the manifest.
    #[arg(long)]
    pub cranelift: Option<String>,
    /// Build profile recorded in the manifest.
    #[arg(long, default_value = "release")]
    pub profile: String,
    /// Release timestamp (RFC 3339). Defaults to the current time.
    #[arg(long)]
    pub released_at: Option<String>,
    /// GitHub `owner/repo` that hosts the release assets.
    #[arg(long, default_value = "duongonix/vut")]
    pub repo: String,
    /// Base URL for asset links. Defaults to the GitHub release download URL.
    /// Use `file://<dir>` for offline/clean-machine smoke tests.
    #[arg(long)]
    pub url_base: Option<String>,
}
