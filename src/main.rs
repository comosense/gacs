use sha2::{Digest, Sha256, Sha512, digest::Output};
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

use gacs::{Charset, Generator, GeneratorError};

const DEFAULT_CHARSET: &str = "ps";
const DEFAULT_LENGTH: usize = 32;
const FILE_BUFFER_SIZE: usize = 8_192;
const HASH_BYTES: usize = (512 / u8::BITS) as usize;

#[derive(Error, Debug)]
enum GacsError {
    #[error("CLI Eror: {0}")]
    Cli(String),

    #[error("Generator error: {0}")]
    Generator(#[from] GeneratorError),

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
    charset: String,
    file: Option<PathBuf>,
    length: usize,
    rule: Option<String>,
    verbose: bool,
}

impl Args {
    fn parse() -> Result<Self, GacsError> {
        let mut args: std::iter::Skip<std::env::Args> = std::env::args().skip(1);
        let mut seed: Option<String> = None;
        let mut charset: String = String::from(DEFAULT_CHARSET);
        let mut file: Option<PathBuf> = None;
        let mut length: usize = DEFAULT_LENGTH;
        let mut rule: Option<String> = None;
        let mut verbose: bool = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-c" | "--charset" => {
                    let val: String = args.next().ok_or_else(|| {
                        GacsError::Cli(String::from("Missing value for --charset"))
                    })?;
                    if (val != "64") && (val != "us") && (val != "ps") {
                        return Err(GacsError::Cli(format!(
                            "Invalid charset '{val}'. Expected '64', 'us', or 'ps'"
                        )));
                    }
                    charset = val;
                }
                "-f" | "--file" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| GacsError::Cli(String::from("Missing value for --file")))?;
                    file = Some(PathBuf::from(val));
                }
                "-l" | "--length" => {
                    let val: String = args.next().ok_or_else(|| {
                        GacsError::Cli(String::from("Missing value for --length"))
                    })?;
                    length = val.parse::<usize>().map_err(|_| {
                        GacsError::Cli(format!("Invalid length: '{val}' is not a valid number"))
                    })?;
                }
                "-r" | "--rule" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| GacsError::Cli(String::from("Missing value for --rule")))?;
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
                    return Err(GacsError::Cli(format!("Unknown option '{arg}'")));
                }
                _ => {
                    if seed.is_none() {
                        seed = Some(arg);
                    } else {
                        return Err(GacsError::Cli(format!(
                            "Unexpected positional argument '{arg}'"
                        )));
                    }
                }
            }
        }

        Ok(Args {
            seed,
            charset,
            file,
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
        DEFAULT_CHARSET
    );
    println!("  -f, --file <FILE>      Optional file to use as an additional cryptographic salt");
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

fn gen_seed() -> Result<String, GacsError> {
    let time_hash: Output<Sha256> = Sha256::digest(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string()
            .as_bytes(),
    );

    Ok(hex::encode(time_hash))
}

fn gen_hash(seed: &str, file: Option<&PathBuf>) -> Result<[u8; HASH_BYTES], GacsError> {
    let mut hasher: Sha512 = Sha512::new();

    hasher.update(seed.as_bytes());

    if let Some(path) = file {
        let mut f: File = File::open(path)?;
        let mut buf: [u8; FILE_BUFFER_SIZE] = [0u8; FILE_BUFFER_SIZE];
        loop {
            let cnt: usize = f.read(&mut buf)?;
            if cnt == 0 {
                break;
            }
            hasher.update(&buf[..cnt]);
        }
    }

    Ok(hasher.finalize().into())
}

fn run() -> Result<(), GacsError> {
    let args: Args = Args::parse()?;

    let generator: Generator = Generator::build(
        match args.charset.as_str() {
            "64" => &Charset::Base64,
            "us" => &Charset::UrlSafe,
            "ps" => &Charset::PasswordSafe,
            _ => unreachable!(),
        },
        args.rule.as_ref(),
    )?;

    let seed: String = match args.seed {
        Some(s) => s,
        None => gen_seed()?,
    };

    let file: Option<&PathBuf> = args.file.as_ref();

    let generated: String = generator.map(&gen_hash(&seed, file)?, args.length)?;
    if args.verbose {
        println!("[Seed] {}", seed);
        if let Some(path) = file {
            println!("[File(salt)] {}", path.display());
        } else {
            println!("[File(salt)] (none)");
        }
        println!("[Length] {}", args.length);
        println!(
            "[Character set] {}",
            std::str::from_utf8(generator.charset())?
        );
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
