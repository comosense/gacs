use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use gacs::{Charset, Gacs, GacsError};

const CLI_CHARSET_64: &str = "64";
const CLI_CHARSET_US: &str = "us";
const CLI_CHARSET_PS: &str = "ps";

const DEFAULT_CHARSET: Charset = Charset::PasswordSafe;
const DEFAULT_LENGTH: usize = 32;

#[derive(Error, Debug)]
enum CliError {
    #[error("Missing value for {0}")]
    MissingVal(String),

    #[error("Invalid value for {0}: '{1}'")]
    InvalidVal(String, String),

    #[error("Conflicting arguments: {0} and {1}")]
    ConflictOpt(String, String),

    #[error("Unknown option '{0}'")]
    UnknownOpt(String),

    #[error("Unexpected positional argument '{0}'")]
    UnexpectedPos(String),

    #[error("Gacs error: {0}")]
    Gacs(#[from] GacsError),

    #[error("Failed to show charset: {0}")]
    Str(#[from] std::str::Utf8Error),

    #[error("Failed to get System Time: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

enum SeedFrom {
    Cli,
    Auto,
}

struct Seed {
    seed: String,
    from: SeedFrom,
}

struct Args {
    auto_seed: String,
    seed: Option<String>,
    salt: Option<PathBuf>,
    length: usize,
    charset: Charset,
    rule: Option<String>,
    number: Option<usize>,
    verbose: bool,
}

impl Args {
    fn parse() -> Result<Self, CliError> {
        let mut args: std::iter::Skip<std::env::Args> = std::env::args().skip(1);
        let mut seed: Option<String> = None;
        let mut salt: Option<PathBuf> = None;
        let mut length: usize = DEFAULT_LENGTH;
        let mut charset: Charset = DEFAULT_CHARSET;
        let mut rule: Option<String> = None;
        let mut number: Option<usize> = None;
        let mut verbose: bool = false;

        while let Some(a) = args.next() {
            match a.as_str() {
                "-s" | "--salt" => {
                    let val: String = args.next().ok_or(CliError::MissingVal(a))?;
                    salt = Some(PathBuf::from(&val));
                }
                "-l" | "--length" => {
                    let val: String = args.next().ok_or(CliError::MissingVal(a.clone()))?;
                    length = val.parse().map_err(|_| CliError::InvalidVal(a, val))?;
                }
                "-c" | "--charset" => {
                    let val: String = args.next().ok_or(CliError::MissingVal(a.clone()))?;
                    charset = match val.as_str() {
                        CLI_CHARSET_64 => Charset::Base64,
                        CLI_CHARSET_US => Charset::UrlSafe,
                        CLI_CHARSET_PS => Charset::PasswordSafe,
                        _ => return Err(CliError::InvalidVal(a, val)),
                    };
                }
                "-r" | "--rule" => {
                    let val: String = args.next().ok_or(CliError::MissingVal(a))?;
                    rule = Some(val);
                }
                "-n" | "--number" => {
                    let val: String = args.next().ok_or(CliError::MissingVal(a.clone()))?;
                    number = Some(val.parse().map_err(|_| CliError::InvalidVal(a, val))?);
                }
                "-v" | "--verbose" => {
                    verbose = true;
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                _ if a.starts_with('-') => {
                    return Err(CliError::UnknownOpt(a));
                }
                _ => {
                    if seed.is_none() {
                        seed = Some(a);
                    } else {
                        return Err(CliError::UnexpectedPos(a));
                    }
                }
            }
        }

        if number.is_some() && seed.is_some() {
            return Err(CliError::ConflictOpt(
                String::from("SEED"),
                String::from("--number"),
            ));
        }

        Ok(Args {
            auto_seed: String::new(),
            seed,
            salt,
            length,
            charset,
            rule,
            number,
            verbose,
        })
    }

    fn get_or_generate_seed(&mut self) -> Result<Seed, CliError> {
        match &self.seed {
            Some(s) => Ok(Seed {
                seed: s.clone(),
                from: SeedFrom::Cli,
            }),
            None => {
                let auto_base: String = SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_nanos()
                    .to_string()
                    + &self.auto_seed;
                self.auto_seed =
                    Gacs::build(&Charset::UrlSafe, Some("-:@"))?.generate(&auto_base, None, 0)?;
                Ok(Seed {
                    seed: self.auto_seed.clone(),
                    from: SeedFrom::Auto,
                })
            }
        }
    }
}

fn print_help() {
    print!(
        include_str!("help.txt"),
        pkg_name = env!("CARGO_PKG_NAME"),
        d_len = DEFAULT_LENGTH,
        cs_64 = CLI_CHARSET_64,
        cs_us = CLI_CHARSET_US,
        cs_ps = CLI_CHARSET_PS,
        d_cs = match DEFAULT_CHARSET {
            Charset::Base64 => CLI_CHARSET_64,
            Charset::UrlSafe => CLI_CHARSET_US,
            Charset::PasswordSafe => CLI_CHARSET_PS,
        },
    );
}

fn run() -> Result<(), CliError> {
    let mut args: Args = Args::parse()?;

    let gacs: Gacs = Gacs::build(&args.charset, args.rule.as_deref())?;

    for _ in 0..args.number.unwrap_or(1) {
        let seed: Seed = args.get_or_generate_seed()?;
        let generated: String = gacs.generate(&seed.seed, args.salt.as_deref(), args.length)?;

        println!("{}", generated);
        if args.verbose {
            eprintln!(
                "  [SEED{}] {}",
                match seed.from {
                    SeedFrom::Cli => "",
                    SeedFrom::Auto => "(Auto)",
                },
                seed.seed
            );
        }
    }
    if args.verbose {
        if let Some(p) = &args.salt {
            eprintln!("  [SALT] {}", p.display());
        }
        eprintln!("  [LENGTH] {}", args.length);
        eprintln!("  [CHARSET] {}\n", std::str::from_utf8(gacs.tbl())?);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
