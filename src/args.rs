use std::path::{Path, PathBuf};

use lexopt::prelude::*;
use thiserror::Error;

use gacs::Charset;

#[derive(Error, Debug)]
pub enum ArgsError {
    #[error(transparent)]
    Lexopt(#[from] lexopt::Error),

    #[error("invalid charset: '{0}'")]
    InvalidCharset(String),

    #[error("conflicting arguments: {0} and {1}")]
    ConflictOpts(String, String),
}

pub struct Args {
    seed: Option<String>,
    salt: Option<PathBuf>,
    length: Option<usize>,
    charset: Charset,
    rule: Option<String>,
    count: Option<usize>,
    seed_length: Option<usize>,
    verbose: bool,
}

impl Args {
    const DEFAULT_CHARSET: Charset = Charset::PasswordSafe;
    const DEFAULT_LENGTH: usize = 32;

    const CLI_CHARSET_64: &str = "64";
    const CLI_CHARSET_US: &str = "us";
    const CLI_CHARSET_PS: &str = "ps";
    const CLI_CHARSET_SS: &str = "ss";

    pub fn parse() -> Result<Option<Args>, ArgsError> {
        let mut seed: Option<String> = None;
        let mut salt: Option<PathBuf> = None;
        let mut length: Option<usize> = Some(Self::DEFAULT_LENGTH);
        let mut charset: Charset = Self::DEFAULT_CHARSET;
        let mut rule: Option<String> = None;
        let mut count: Option<usize> = None;
        let mut seed_length: Option<usize> = None;
        let mut verbose: bool = false;

        let mut parser: lexopt::Parser = lexopt::Parser::from_env();
        while let Some(arg) = parser.next()? {
            match arg {
                Short('s') | Long("salt") => {
                    salt = Some(parser.value()?.parse()?);
                }
                Short('l') | Long("length") => {
                    length = Some(parser.value()?.parse()?);
                }
                Short('c') | Long("charset") => {
                    charset = parser.value()?.parse_with(Self::get_charset)?;
                }
                Short('r') | Long("rule") => {
                    rule = Some(parser.value()?.parse()?);
                }
                Short('n') | Long("count") => {
                    count = Some(parser.value()?.parse()?);
                }
                Short('L') | Long("seed-length") => {
                    seed_length = Some(parser.value()?.parse()?);
                }
                Short('v') | Long("verbose") => {
                    verbose = true;
                }
                Short('h') | Long("help") => {
                    Self::print_help();
                    return Ok(None);
                }
                Short('V') | Long("version") => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                Value(val) if seed.is_none() => {
                    seed = Some(val.parse()?);
                }
                _ => return Err(ArgsError::Lexopt(arg.unexpected())),
            }
        }

        if seed.is_some() && count.is_some() {
            return Err(ArgsError::ConflictOpts(
                String::from("SEED"),
                String::from("-n | --count"),
            ));
        }

        if seed.is_some() && seed_length.is_some() {
            return Err(ArgsError::ConflictOpts(
                String::from("SEED"),
                String::from("-L | --seed-length"),
            ));
        }

        Ok(Some(Args {
            seed,
            salt,
            length,
            charset,
            rule,
            count,
            seed_length,
            verbose,
        }))
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

    fn get_charset(cli_charset: &str) -> Result<Charset, ArgsError> {
        match cli_charset {
            Self::CLI_CHARSET_64 => Ok(Charset::Base64),
            Self::CLI_CHARSET_US => Ok(Charset::UrlSafe),
            Self::CLI_CHARSET_PS => Ok(Charset::PasswordSafe),
            Self::CLI_CHARSET_SS => Ok(Charset::ShellSafe),
            other => Err(ArgsError::InvalidCharset(other.to_string())),
        }
    }

    pub fn seed(&self) -> Option<&str> {
        self.seed.as_deref()
    }

    pub fn salt(&self) -> Option<&Path> {
        self.salt.as_deref()
    }

    pub fn length(&self) -> Option<usize> {
        self.length
    }

    pub fn charset(&self) -> &Charset {
        &self.charset
    }

    pub fn rule(&self) -> Option<&str> {
        self.rule.as_deref()
    }

    pub fn count(&self) -> Option<usize> {
        self.count
    }

    pub fn seed_length(&self) -> Option<usize> {
        self.seed_length
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }
}
