use std::path::PathBuf;

use gacs::Charset;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArgsError {
    #[error("Missing value for {0}")]
    MissingVal(String),

    #[error("Invalid value for {0}: '{1}'")]
    InvalidVal(String, String),

    #[error("Conflicting arguments: {0} and {1}")]
    ConflictOpts(String, String),

    #[error("Unknown option '{0}'")]
    UnknownOpt(String),

    #[error("Unexpected positional argument '{0}'")]
    UnexpectedPos(String),

    #[error("Help requested")]
    Help,

    #[error("Version requested")]
    Version,
}

pub struct Args {
    seed: Option<String>,
    salt: Option<PathBuf>,
    length: Option<usize>,
    charset: Charset,
    rule: Option<String>,
    number: Option<usize>,
    slength: Option<usize>,
    verbose: bool,
}

impl Args {
    const CLI_CHARSET_64: &str = "64";
    const CLI_CHARSET_US: &str = "us";
    const CLI_CHARSET_PS: &str = "ps";
    const CLI_CHARSET_SS: &str = "ss";

    const DEFAULT_CHARSET: Charset = Charset::PasswordSafe;
    const DEFAULT_LENGTH: usize = 32;

    pub fn parse() -> Result<Self, ArgsError> {
        let mut args: std::iter::Skip<std::env::Args> = std::env::args().skip(1);
        let mut seed: Option<String> = None;
        let mut salt: Option<PathBuf> = None;
        let mut length: Option<usize> = Some(Self::DEFAULT_LENGTH);
        let mut charset: Charset = Self::DEFAULT_CHARSET;
        let mut rule: Option<String> = None;
        let mut number: Option<usize> = None;
        let mut slength: Option<usize> = None;
        let mut verbose: bool = false;

        while let Some(a) = args.next() {
            match a.as_str() {
                "-s" | "--salt" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    salt = Some(PathBuf::from(&val));
                }
                "-l" | "--length" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    length = Some(val.parse().map_err(|_| ArgsError::InvalidVal(a, val))?);
                }
                "-c" | "--charset" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    charset = match val.as_str() {
                        Self::CLI_CHARSET_64 => Charset::Base64,
                        Self::CLI_CHARSET_US => Charset::UrlSafe,
                        Self::CLI_CHARSET_PS => Charset::PasswordSafe,
                        Self::CLI_CHARSET_SS => Charset::ShellSafe,
                        _ => return Err(ArgsError::InvalidVal(a, val)),
                    };
                }
                "-r" | "--rule" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    rule = Some(val);
                }
                "-N" | "--number" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    number = Some(val.parse().map_err(|_| ArgsError::InvalidVal(a, val))?);
                }
                "-L" | "--slength" => {
                    let val: String = args
                        .next()
                        .ok_or_else(|| ArgsError::MissingVal(a.clone()))?;
                    slength = Some(val.parse().map_err(|_| ArgsError::InvalidVal(a, val))?);
                }
                "-v" | "--verbose" => {
                    verbose = true;
                }
                "-h" | "--help" => {
                    Self::print_help();
                    return Err(ArgsError::Help);
                }
                "-V" | "--version" => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    return Err(ArgsError::Version);
                }
                _ if a.starts_with('-') => {
                    return Err(ArgsError::UnknownOpt(a));
                }
                _ => {
                    if seed.is_none() {
                        seed = Some(a);
                    } else {
                        return Err(ArgsError::UnexpectedPos(a));
                    }
                }
            }
        }

        if seed.is_some() && number.is_some() {
            return Err(ArgsError::ConflictOpts(
                String::from("SEED"),
                String::from("-N | --number"),
            ));
        }

        if seed.is_some() && slength.is_some() {
            return Err(ArgsError::ConflictOpts(
                String::from("SEED"),
                String::from("-L | --slength"),
            ));
        }

        Ok(Args {
            seed,
            salt,
            length,
            charset,
            rule,
            number,
            slength,
            verbose,
        })
    }

    fn print_help() {
        print!(
            include_str!("help.txt"),
            pkg_name = env!("CARGO_PKG_NAME"),
            d_len = Self::DEFAULT_LENGTH,
            cs_64 = Self::CLI_CHARSET_64,
            cs_us = Self::CLI_CHARSET_US,
            cs_ps = Self::CLI_CHARSET_PS,
            cs_ss = Self::CLI_CHARSET_SS,
            d_cs = match Self::DEFAULT_CHARSET {
                Charset::Base64 => Self::CLI_CHARSET_64,
                Charset::UrlSafe => Self::CLI_CHARSET_US,
                Charset::PasswordSafe => Self::CLI_CHARSET_PS,
                Charset::ShellSafe => Self::CLI_CHARSET_SS,
            },
        );
    }

    pub fn seed(&self) -> &Option<String> {
        &self.seed
    }

    pub fn salt(&self) -> &Option<PathBuf> {
        &self.salt
    }

    pub fn length(&self) -> Option<usize> {
        self.length
    }

    pub fn charset(&self) -> &Charset {
        &self.charset
    }

    pub fn rule(&self) -> &Option<String> {
        &self.rule
    }

    pub fn number(&self) -> Option<usize> {
        self.number
    }

    pub fn slength(&self) -> Option<usize> {
        self.slength
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }
}
