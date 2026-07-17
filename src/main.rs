use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use gacs::{Charset, Gacs, GacsError};

mod args;

use crate::args::{Args, ArgsError};

#[derive(Error, Debug)]
enum MainError {
    #[error(transparent)]
    Args(#[from] ArgsError),

    #[error(transparent)]
    Gacs(#[from] GacsError),

    #[error("Failed to get System Time: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),

    #[error("Failed to show charset: {0}")]
    Str(#[from] std::str::Utf8Error),
}

fn make_seed(length: Option<usize>, uniq: usize) -> Result<String, MainError> {
    let sb: String = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string()
        + &uniq.to_string();
    Ok(Gacs::build(&Charset::ShellSafe, None)?.generate(&sb, None, length)?)
}

fn run() -> Result<(), MainError> {
    let args: Args = Args::parse()?;

    let salt: Option<&Path> = args.salt();
    let length: Option<usize> = args.length();
    let charset: &gacs::Charset = args.charset();
    let rule: Option<&str> = args.rule();
    let verbose: bool = args.verbose();

    let gacs: Gacs = Gacs::build(charset, rule)?;

    match args.seed() {
        Some(s) => {
            let generated: String = gacs.generate(s, salt, length)?;
            println!("{}", generated);
            if verbose {
                eprintln!("  [SEED] {}", s);
            }
        }
        None => {
            let number: usize = args.number().unwrap_or(1);
            let slength: Option<usize> = args.slength();
            for i in 0..number {
                let seed: String = make_seed(slength, i)?;
                let generated: String = gacs.generate(&seed, salt, length)?;
                println!("{}", generated);
                if verbose {
                    eprintln!("  [SEED(Auto)] {}", seed);
                }
            }
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
