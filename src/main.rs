use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256, Sha512, digest::Output};
use std::{
    fmt::Write,
    fs::File,
    io::Read,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const TABLE_LEN: usize = 1 << 6;
const CHARSET_STYLE: [[u8; TABLE_LEN]; 3] = [
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", // BASE64
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_", // URL safe
    *b"ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_", // Password safe
];
const RULE_DELIMITER: char = ':';
const FILE_BUFFER_SIZE: usize = 8_192;
const U32_BYTES_LEN: usize = ((u32::BITS) / (u8::BITS)) as usize;

const A: u64 = 1_664_525; // [alt1] 1103515245, [alt2] 214013, [alt3] 25214903917
const C: u64 = 1_013_904_223; // [alt1] 12345, [alt2] 2531011, [alt3] 11
const M: u64 = 1 << 32; // [alt1] 1 << 31, [alt2] 1 << 31, [alt3] 1 << 48

#[derive(Debug)]
enum TblError {
    InvalidReplaceRule,
    InvalidReplaceChars,
    CharsetUtf8ParseError(std::str::Utf8Error),
    InvalidSource,
    SourceSliceError(std::array::TryFromSliceError),
    LengthExceedsMaximum(usize, usize),
    OutputUtf8ParseError(std::string::FromUtf8Error),
}

impl std::fmt::Display for TblError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TblError::InvalidReplaceRule => {
                write!(
                    f,
                    "invalid replace rule: expected 'target:replacement' (e.g., 'Zz9:^&*')"
                )
            }
            TblError::InvalidReplaceChars => write!(
                f,
                "invalid replace characters: contains duplicates or characters not found in the base charset"
            ),
            TblError::CharsetUtf8ParseError(e) => {
                write!(f, "failed to decode charset as UTF-8: {e}")
            }
            TblError::InvalidSource => write!(f, "invalid source data: length is too short"),
            TblError::SourceSliceError(e) => write!(f, "failed to slice source: {e}"),
            TblError::LengthExceedsMaximum(req, max) => {
                write!(
                    f,
                    "requested length ({req}) exceeds the maximum possible length ({max})"
                )
            }
            TblError::OutputUtf8ParseError(e) => {
                write!(f, "failed to decode mapped output as UTF-8: {e}")
            }
        }
    }
}

impl std::error::Error for TblError {}

#[derive(Debug)]
enum GacsError {
    Tbl(TblError),
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    SystemTime(std::time::SystemTimeError),
}

impl std::fmt::Display for GacsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GacsError::Tbl(e) => write!(f, "Table error: {e}"),
            GacsError::Io(e) => write!(f, "Io error: {e}"),
            GacsError::Fmt(e) => write!(f, "Formatting error: {e}"),
            GacsError::SystemTime(e) => write!(f, "System time error: {e}"),
        }
    }
}

impl std::error::Error for GacsError {}

impl From<TblError> for GacsError {
    fn from(err: TblError) -> Self {
        GacsError::Tbl(err)
    }
}

impl From<std::io::Error> for GacsError {
    fn from(err: std::io::Error) -> Self {
        GacsError::Io(err)
    }
}

impl From<std::fmt::Error> for GacsError {
    fn from(err: std::fmt::Error) -> Self {
        GacsError::Fmt(err)
    }
}

impl From<std::time::SystemTimeError> for GacsError {
    fn from(err: std::time::SystemTimeError) -> Self {
        GacsError::SystemTime(err)
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum CharsetStyle {
    #[clap(name = "64")]
    Base64,
    #[clap(name = "us")]
    UrlSafe,
    #[clap(name = "ps")]
    PasswordSafe,
}

impl CharsetStyle {
    fn charset(&self) -> &[u8; TABLE_LEN] {
        match self {
            Self::Base64 => &CHARSET_STYLE[0],
            Self::UrlSafe => &CHARSET_STYLE[1],
            Self::PasswordSafe => &CHARSET_STYLE[2],
        }
    }
}

impl std::fmt::Display for CharsetStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharsetStyle::Base64 => write!(f, "64"),
            CharsetStyle::UrlSafe => write!(f, "us"),
            CharsetStyle::PasswordSafe => write!(f, "ps"),
        }
    }
}

/// A secure, deterministic ASCII character generator.
///
/// Generates reproducible characters based on a given seed,
/// an optional salt file, and specific character sets.
#[derive(Parser)]
#[command(author, version)]
struct Args {
    /// Base string to generate the characters from
    #[arg(value_name = "SEED")]
    seed: Option<String>,

    /// Character set style to use
    /// (64: BASE64 / us: URL safe / ps: Password safe)
    #[arg(short, long, value_name = "STYLE", default_value_t = CharsetStyle::PasswordSafe)]
    charset: CharsetStyle,

    /// Optional file to use as an additional cryptographic salt
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Length of the generated characters
    #[arg(short, long, value_name = "LENGTH", default_value_t = 32)]
    length: usize,

    /// Replace specific characters in the charset (Format: 'target:replacement')
    /// Example: -r 'Zz9:^&*' replaces 'Z', 'z', and '9' with '^', '&', and '*'
    #[arg(short, long, value_name = "RULE")]
    rule: Option<String>,

    /// Print detailed configuration along with the generated characters
    #[arg(short, long, default_value_t = false)]
    detail: bool,
}

struct Tbl {
    charset: [u8; TABLE_LEN],
}

impl Tbl {
    fn new(charset_style: &CharsetStyle, rule: Option<&String>) -> Result<Self, TblError> {
        let charset: [u8; TABLE_LEN] = match rule {
            Some(r) => {
                let (d, e) = r
                    .split_once(RULE_DELIMITER)
                    .ok_or(TblError::InvalidReplaceRule)?;
                if !d.is_ascii() || !e.is_ascii() || d.len() != e.len() {
                    return Err(TblError::InvalidReplaceRule);
                }
                charset_style
                    .charset()
                    .iter()
                    .copied()
                    .filter(|&c| !d.as_bytes().contains(&c))
                    .chain(e.bytes())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .map_err(|_| TblError::InvalidReplaceChars)?
            }
            None => *charset_style.charset(),
        };

        Ok(Self { charset })
    }

    fn get_charset_str(&self) -> Result<&str, TblError> {
        std::str::from_utf8(&self.charset).map_err(TblError::CharsetUtf8ParseError)
    }

    fn map(&self, src: &[u8], len: usize) -> Result<String, TblError> {
        let (s_bytes, p_bytes) = src
            .split_at_checked(U32_BYTES_LEN)
            .ok_or(TblError::InvalidSource)?;
        let scrambler: u32 =
            u32::from_be_bytes(s_bytes.try_into().map_err(TblError::SourceSliceError)?);
        let map_len: usize = (p_bytes.len() * 4).div_ceil(3);
        if len > map_len {
            return Err(TblError::LengthExceedsMaximum(len, map_len));
        }

        let s_charset: [u8; TABLE_LEN] = self.scramble(scrambler);
        let mut mapped: Vec<u8> = Vec::with_capacity(map_len);

        for chunk in p_bytes.chunks(3) {
            match chunk {
                [b0, b1, b2] => {
                    mapped.push(s_charset[(b0 >> 2) as usize]);
                    mapped.push(s_charset[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
                    mapped.push(s_charset[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
                    mapped.push(s_charset[(b2 & 0x3f) as usize]);
                }
                [b0, b1] => {
                    mapped.push(s_charset[(b0 >> 2) as usize]);
                    mapped.push(s_charset[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
                    mapped.push(s_charset[((b1 & 0x0f) << 2) as usize]);
                }
                [b0] => {
                    mapped.push(s_charset[(b0 >> 2) as usize]);
                    mapped.push(s_charset[((b0 & 0x03) << 4) as usize]);
                }
                _ => unreachable!(),
            }
        }
        mapped.truncate(len);

        String::from_utf8(mapped).map_err(TblError::OutputUtf8ParseError)
    }

    fn scramble(&self, scrambler: u32) -> [u8; TABLE_LEN] {
        let mut scrambled: [u8; TABLE_LEN] = self.charset;
        let mut s_rand: u64 = scrambler as u64;

        for i in 0..scrambled.len() {
            s_rand = (A * s_rand + C) % M;
            scrambled.swap(i, (s_rand as usize) % (i + 1));
        }

        scrambled
    }
}

fn gen_seed() -> Result<String, GacsError> {
    let time_hash: Output<Sha256> = Sha256::digest(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string()
            .as_bytes(),
    );

    let mut seed: String = String::with_capacity(time_hash.len() * 2);
    for b in time_hash {
        write!(&mut seed, "{:02x}", b)?
    }

    Ok(seed)
}

fn gen_hash(seed: &str, file: Option<&PathBuf>) -> Result<Vec<u8>, GacsError> {
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

    Ok(hasher.finalize().to_vec())
}

fn run() -> Result<(), GacsError> {
    let args: Args = Args::parse();

    let tbl: Tbl = Tbl::new(&args.charset, args.rule.as_ref())?;

    let seed: String = match args.seed {
        Some(s) => s,
        None => gen_seed()?,
    };

    let file: Option<&PathBuf> = args.file.as_ref();
    let generated: String = tbl.map(&gen_hash(&seed, file)?, args.length)?;

    if args.detail {
        println!("[Seed] {}", seed);
        if let Some(path) = file {
            println!("[File(salt)] {}", path.display());
        } else {
            println!("[File(salt)] (none)");
        }
        println!("[Length] {}", args.length);
        println!("[Character set] {}", tbl.get_charset_str()?);
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
