use std::{
    path::Path,
    process::ExitCode,
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

    #[error(transparent)]
    Time(#[from] std::time::SystemTimeError),

    #[error(transparent)]
    Str(#[from] std::str::Utf8Error),
}

fn seed_base(uniq: usize) -> Result<String, MainError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string()
        + &std::process::id().to_string()
        + &uniq.to_string())
}

fn run(args: Args) -> Result<(), MainError> {
    let salt: Option<&Path> = args.salt();
    let length: Option<usize> = args.length();
    let charset: &gacs::Charset = args.charset();
    let rule: Option<&str> = args.rule();
    let verbose: bool = args.verbose();

    let gacs: Gacs = Gacs::build(charset, rule)?;

    match args.seed() {
        Some(seed) => {
            let generated: String = gacs.generate(seed, salt, length)?;
            println!("{}", generated);
            if verbose {
                eprintln!("  [SEED] {}", seed);
            }
        }
        None => {
            let seeder: Gacs = Gacs::build(&Charset::ShellSafe, None)?;
            let seed_length: Option<usize> = args.seed_length();

            let count: usize = args.count().unwrap_or(1);
            for i in 0..count {
                let auto_seed: String = seeder.generate(&seed_base(i)?, None, seed_length)?;
                let generated: String = gacs.generate(&auto_seed, salt, length)?;
                println!("{}", generated);
                if verbose {
                    eprintln!("  [SEED(Auto)] {}", auto_seed);
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

fn main() -> ExitCode {
    if let Err(e) = match Args::parse() {
        Ok(Some(a)) => run(a),
        Ok(None) => Ok(()),
        Err(e) => Err(MainError::Args(e)),
    } {
        eprintln!("{}: {}", env!("CARGO_PKG_NAME"), e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
