use std::path::Path;

use thiserror::Error;

use gacs::{Gacs, GacsError};

mod args;
mod seed;

use crate::{
    args::{Args, ArgsError},
    seed::{Seed, SeedError, SeedFrom},
};

#[derive(Error, Debug)]
enum MainError {
    #[error(transparent)]
    Args(#[from] ArgsError),

    #[error(transparent)]
    Seed(#[from] SeedError),

    #[error(transparent)]
    Gacs(#[from] GacsError),

    #[error("Failed to show charset: {0}")]
    Str(#[from] std::str::Utf8Error),
}

fn run() -> Result<(), MainError> {
    let args: Args = Args::parse()?;

    let mut seed: Seed = Seed::new(args.seed().as_deref());
    let salt: Option<&Path> = args.salt().as_deref();
    let length: Option<usize> = *args.length();
    let charset: &gacs::Charset = args.charset();
    let rule: Option<&str> = args.rule().as_deref();
    let number: usize = args.number().unwrap_or(1);
    let verbose: bool = *args.verbose();

    let gacs: Gacs = Gacs::build(charset, rule)?;

    for _ in 0..number {
        seed.update()?;
        let generated: String = gacs.generate(seed.seed(), salt, length)?;

        println!("{}", generated);
        if verbose {
            eprintln!(
                "  [SEED{}] {}",
                match seed.from() {
                    SeedFrom::Cli => "",
                    SeedFrom::Auto => "(Auto)",
                },
                seed.seed()
            );
        }
    }

    if verbose {
        if let Some(p) = salt {
            eprintln!("  [SALT] {}", p.display());
        }
        if let Some(l) = length {
            eprintln!("  [LENGTH] {}", l);
        }
        eprintln!("  [CHARSET] {}\n", std::str::from_utf8(gacs.tbl())?);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        match e {
            MainError::Args(ArgsError::Help) | MainError::Args(ArgsError::Version) => {
                std::process::exit(0)
            }
            _ => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}
