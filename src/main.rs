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
    #[error("Missing value for --charset")]
    MissingCharset,

    #[error("Invalid charset '{0}'. Expected '{1}'")]
    InvalidCharset(String, String),

    #[error("Missing value for --salt")]
    MissingSalt,

    #[error("Missing value for --length")]
    MissingLength,

    #[error("Invalid length: '{0}' is not a valid number")]
    InvalidLength(String),

    #[error("Missing value for --rule")]
    MissingRule,

    #[error("Unknown option '{0}'")]
    UnknownOpt(String),

    #[error("Unexpected positional argument '{0}'")]
    UnexpectedPos(String),

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
                    let val: String = args.next().ok_or(CliError::MissingCharset)?;
                    charset = get_charset(&val)?;
                }
                "-s" | "--salt" => {
                    let val: String = args.next().ok_or(CliError::MissingSalt)?;
                    salt = Some(PathBuf::from(&val));
                }
                "-l" | "--length" => {
                    let val: String = args.next().ok_or(CliError::MissingLength)?;
                    length = val.parse().map_err(|_| CliError::InvalidLength(val))?;
                }
                "-r" | "--rule" => {
                    let val: String = args.next().ok_or(CliError::MissingRule)?;
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
                    return Err(CliError::UnknownOpt(arg));
                }
                _ => {
                    if seed.is_none() {
                        seed = Some(arg);
                    } else {
                        return Err(CliError::UnexpectedPos(arg));
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
        "  -c, --charset <CHARSET>   Character set to use ({}, {}, {}) [default: {}]",
        CLI_CHARSET_64,
        CLI_CHARSET_US,
        CLI_CHARSET_PS,
        get_cli_charset(&DEFAULT_CHARSET)
    );
    println!(
        "  -s, --salt <FILE>         Optional file to use as an additional cryptographic salt"
    );
    println!(
        "  -l, --length <LENGTH>     Length of the generated characters [default: {}]",
        DEFAULT_LENGTH
    );
    println!(
        "  -r, --rule <RULE>         Replace specific characters in the charset (Format: 'target:replacement')"
    );
    println!(
        "  -v, --verbose             Print detailed configuration along with the generated characters"
    );
    println!("  -h, --help                Print help");
    println!("  -V, --version             Print version");
}

fn get_charset(cli_charset: &str) -> Result<Charset, CliError> {
    match cli_charset {
        CLI_CHARSET_64 => Ok(Charset::Base64),
        CLI_CHARSET_US => Ok(Charset::UrlSafe),
        CLI_CHARSET_PS => Ok(Charset::PasswordSafe),
        _ => Err(CliError::InvalidCharset(
            String::from(cli_charset),
            format!("{CLI_CHARSET_64}, {CLI_CHARSET_US}, {CLI_CHARSET_PS}"),
        )),
    }
}

fn get_cli_charset(charset: &Charset) -> &str {
    match charset {
        Charset::Base64 => CLI_CHARSET_64,
        Charset::UrlSafe => CLI_CHARSET_US,
        Charset::PasswordSafe => CLI_CHARSET_PS,
    }
}

fn gen_seed(charset: &Charset) -> Result<String, CliError> {
    let seed_base: String = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string()
        + &std::process::id().to_string();

    Ok(Gacs::build(charset, None)?.generate(&seed_base, None, 0)?)
}

fn run() -> Result<(), CliError> {
    let args: Args = Args::parse()?;

    let gacs: Gacs = Gacs::build(&args.charset, args.rule.as_deref())?;

    let seed: String = match args.seed {
        Some(s) => s,
        None => gen_seed(&args.charset)?,
    };

    let generated: String = gacs.generate(&seed, args.salt.as_deref(), args.length)?;

    println!("{}", generated);
    if args.verbose {
        eprintln!(" [SEED] {}", seed);
        if let Some(p) = args.salt {
            eprintln!(" [SALT] {}", p.display());
        }
        eprintln!(" [LENGTH] {}", args.length);
        eprintln!(" [CHARSET] {}", std::str::from_utf8(gacs.tbl())?);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
