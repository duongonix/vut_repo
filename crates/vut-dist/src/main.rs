//! Distribution packaging tool for Vut.
mod archive;
mod checksum;
mod cli;
mod manifest;
mod stage;

use clap::Parser as _;

fn main() {
    if let Err(error) = run() {
        eprintln!("vut-dist: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();
    let artifact = stage::assemble(&args)?;
    println!("archive:  {}", artifact.archive.display());
    println!("sha256:   {}", artifact.checksum);
    println!("staging:  {}", artifact.staging.display());
    Ok(())
}
