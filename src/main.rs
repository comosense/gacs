use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use thiserror::Error;

use gacs::{Charset, Gacs, GacsError};

const OP_CHARSET_64: &str = "64";
const OP_CHARSET_US: &str = "us";
const OP_CHARSET_PS: &str = "ps";

const DEFAULT_CHARSET: Charset = Charset::PasswordSafe;
const DEFAULT_LENGTH: usize = 32;

#[derive(Error, Debug)]
enum CliError {
    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Gacs error: {0}")]
    Gacs(#[from] GacsError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Fmt error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("str error: {0}")]
    Str(#[from] std::str::Utf8Error),

    #[error("time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

struct Args {
    seed: Option<String>,
    charset: Charset,
    salt: Option<PathBuf>,
    length: usize,
    rule: Option<String>,
    verbose: bool,
}

impl Args {
    fn parse() -> Result<Self, CliError> {
        let mut args: std::iter::Skip<std::env::Args> = std::env::args().skip(1);
        let mut seed: Option<String> = None;
        let mut charset: Charset = DEFAULT_CHARSET;
        let mut salt: Option<PathBuf> = None;
        let mut length: usize = DEFAULT_LENGTH;
        let mut rule: Option<String> = None;
        let mut verbose: bool = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-c" | "--charset" => {
                    let val: String = args.next().ok_or_else(|| {
                        CliError::Cli(String::from("Missing value for --charset"))
                    })?;
                    charset = match val.as_str() {
                        OP_CHARSET_64 => Charset::Base64,
                        OP_CHARSET_US => Charset::UrlSafe,
                        OP_CHARSET_PS => Charset::PasswordSafe,
                        _ => {
                            return Err(CliError::Cli(format!(
                                "Invalid charset '{val}'. Expected '{OP_CHARSET_64}', '{OP_CHARSET_US}', or '{OP_CHARSET_PS}'"
                            )));
                        }
                    };
                }
                "-s" | "--salt" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| CliError::Cli(String::from("Missing value for --salt")))?;
                    salt = Some(PathBuf::from(val));
                }
                "-l" | "--length" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| CliError::Cli(String::from("Missing value for --length")))?;
                    length = val.parse::<usize>().map_err(|_| {
                        CliError::Cli(format!("Invalid length: '{val}' is not a valid number"))
                    })?;
                }
                "-r" | "--rule" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| CliError::Cli(String::from("Missing value for --rule")))?;
                    rule = Some(val);
                }
                "-v" | "--verbose" => {
                    verbose = true;
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("gacs {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                _ if arg.starts_with('-') => {
                    return Err(CliError::Cli(format!("Unknown option '{arg}'")));
                }
                _ => {
                    if seed.is_none() {
                        seed = Some(arg);
                    } else {
                        return Err(CliError::Cli(format!(
                            "Unexpected positional argument '{arg}'"
                        )));
                    }
                }
            }
        }

        Ok(Args {
            seed,
            charset,
            salt,
            length,
            rule,
            verbose,
        })
    }
}

fn print_help() {
    println!("A deterministic ASCII character generator.\n");
    println!("Usage: {} [OPTIONS] [SEED]\n", env!("CARGO_PKG_NAME"));
    println!("Arguments:");
    println!("  [SEED]  Base string to generate the characters from\n");
    println!("Options:");
    println!(
        "  -c, --charset <STYLE>  Character set style to use (64, us, ps) [default: {}]",
        get_op_charset(&DEFAULT_CHARSET)
    );
    println!("  -s, --salt <FILE>      Optional file to use as an additional cryptographic salt");
    println!(
        "  -l, --length <LENGTH>  Length of the generated characters [default: {}]",
        DEFAULT_LENGTH
    );
    println!(
        "  -r, --rule <RULE>      Replace specific characters in the charset (Format: 'target:replacement')"
    );
    println!(
        "  -v, --verbose          Print detailed configuration along with the generated characters"
    );
    println!("  -h, --help             Print help");
    println!("  -V, --version          Print version");
}

fn get_op_charset(charset: &Charset) -> &str {
    match charset {
        Charset::Base64 => OP_CHARSET_64,
        Charset::UrlSafe => OP_CHARSET_US,
        Charset::PasswordSafe => OP_CHARSET_PS,
    }
}

fn gen_seed() -> Result<String, CliError> {
    Ok(hex::encode(Sha256::digest(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_be_bytes(),
    )))
}

fn run() -> Result<(), CliError> {
    let args: Args = Args::parse()?;

    let gacs: Gacs = Gacs::build(&args.charset, args.rule.as_ref())?;

    let seed: String = match args.seed {
        Some(s) => s,
        None => gen_seed()?,
    };
    let generated: String = gacs.generate(&seed, args.salt.as_ref(), args.length)?;

    if args.verbose {
        println!("Seed: {}", seed);
        if let Some(path) = args.salt {
            println!("Salt: {}", path.display());
        } else {
            println!("Salt: (none)");
        }
        println!("Length: {}", args.length);
        println!("Character set: {}", std::str::from_utf8(gacs.tbl())?);
        println!("-> {}", generated);
    } else {
        println!("{}", generated);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
