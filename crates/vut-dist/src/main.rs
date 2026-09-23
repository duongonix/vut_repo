//! Distribution packaging tool for Vut.
mod archive;
mod checksum;
mod cli;
mod manifest;
mod release;
mod stage;
mod version;

use clap::Parser as _;

fn main() {
    if let Err(error) = run() {
        eprintln!("vut-dist: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    match cli::Cli::parse().command {
        cli::Command::Assemble(args) => {
            let artifact = stage::assemble(&args)?;
            println!("archive:  {}", artifact.archive.display());
            println!("sha256:   {}", artifact.checksum);
            println!("staging:  {}", artifact.staging.display());
        }
        cli::Command::ReleaseManifest(args) => {
            let inputs = release::Inputs {
                channel: &args.channel,
                profile: &args.profile,
                cranelift: args.cranelift.as_deref(),
                released_at: args.released_at.as_deref(),
                repo: &args.repo,
                url_base: args.url_base.as_deref(),
            };
            let manifest = release::generate(&args.dir, &args.out, &inputs)?;
            println!("release:  {}", args.out.display());
            println!("version:  {}", manifest.version);
            println!("targets:  {}", manifest.targets.len());
        }
    }
    Ok(())
}
