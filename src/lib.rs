use std::{fs::File, io::Read, path::Path};

use sha2::{Digest, Sha512, digest::Output};
use thiserror::Error;

const TBL_SIZE: usize = 1 << 6;
const BASE_TBLS: [[u8; TBL_SIZE]; 3] = [
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", // BASE64
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_", // URL safe
    *b"ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_", // Password safe
];

const A: u64 = 1_664_525; // [alt1] 1103515245, [alt2] 214013, [alt3] 25214903917
const C: u64 = 1_013_904_223; // [alt1] 12345, [alt2] 2531011, [alt3] 11
const M: u64 = 1 << 32; // [alt1] 1 << 31, [alt2] 1 << 31, [alt3] 1 << 48

const FILE_BUFFER_SIZE: usize = 8_192;
const HASH_SIZE: usize = std::mem::size_of::<Output<Sha512>>();
const RULE_DELIM: char = ':';

#[derive(Error, Debug)]
pub enum GacsError {
    #[error("invalid rule: expected 'target:replacement' (e.g., 'Zz9:^&*')")]
    InvalidRuleFmt,

    #[error("invalid rule: contains duplicates or characters not found in the charset")]
    InvalidRuleChars,

    #[error("requested length ({0}) exceeds the maximum length ({1})")]
    LenExceeded(usize, usize),

    #[error("invalid source data: length is too short")]
    InvalidSrc,

    #[error("failed to slice source: {0}")]
    SliceSrc(std::array::TryFromSliceError),

    #[error("file operation failed: {0}")]
    File(std::io::Error),

    #[error("file operation failed: {0}")]
    InvalidOutput(#[from] std::string::FromUtf8Error),
}

pub enum Charset {
    Base64,
    UrlSafe,
    PasswordSafe,
}

impl Charset {
    fn tbl(&self) -> &[u8; TBL_SIZE] {
        match self {
            Self::Base64 => &BASE_TBLS[0],
            Self::UrlSafe => &BASE_TBLS[1],
            Self::PasswordSafe => &BASE_TBLS[2],
        }
    }
}

pub struct Gacs {
    tbl: [u8; TBL_SIZE],
}

impl Gacs {
    pub fn build(charset: &Charset, rule: Option<&str>) -> Result<Self, GacsError> {
        let tbl: [u8; TBL_SIZE] = match rule {
            Some(r) => {
                let (d, e) = r.split_once(RULE_DELIM).ok_or(GacsError::InvalidRuleFmt)?;
                if (!d.is_ascii()) || (!e.is_ascii()) || (d.len() != e.len()) {
                    return Err(GacsError::InvalidRuleFmt);
                }
                charset
                    .tbl()
                    .iter()
                    .copied()
                    .filter(|&c| !d.as_bytes().contains(&c))
                    .chain(e.bytes())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .map_err(|_| GacsError::InvalidRuleChars)?
            }
            None => *charset.tbl(),
        };

        Ok(Self { tbl })
    }

    pub fn tbl(&self) -> &[u8; TBL_SIZE] {
        &self.tbl
    }

    pub fn generate(
        &self,
        seed: &str,
        salt: Option<&Path>,
        len: usize,
    ) -> Result<String, GacsError> {
        let src: [u8; HASH_SIZE] = self.hash(seed, salt)?;
        let (s_src, c_src) = src
            .split_at_checked(std::mem::size_of::<u32>())
            .ok_or(GacsError::InvalidSrc)?;

        let shuffler: u32 = u32::from_be_bytes(s_src.try_into().map_err(GacsError::SliceSrc)?);
        let s_tbl: [u8; TBL_SIZE] = self.shuffle(shuffler);

        self.map(&s_tbl, c_src, len)
    }

    fn hash(&self, seed: &str, salt: Option<&Path>) -> Result<[u8; HASH_SIZE], GacsError> {
        let mut hasher: Sha512 = Sha512::new();

        hasher.update(seed.as_bytes());

        if let Some(path) = salt {
            let mut f: File = File::open(path).map_err(GacsError::File)?;
            let mut buf: [u8; FILE_BUFFER_SIZE] = [0u8; FILE_BUFFER_SIZE];
            loop {
                let cnt: usize = f.read(&mut buf).map_err(GacsError::File)?;
                if cnt == 0 {
                    break;
                }
                hasher.update(&buf[..cnt]);
            }
        }

        Ok(hasher.finalize().into())
    }

    fn shuffle(&self, shuffler: u32) -> [u8; TBL_SIZE] {
        let mut shuffled: [u8; TBL_SIZE] = self.tbl;
        let mut s_rand: u64 = shuffler as u64;

        for i in 0..shuffled.len() {
            s_rand = (A * s_rand + C) % M;
            shuffled.swap(i, (s_rand as usize) % (i + 1));
        }

        shuffled
    }

    fn map(&self, tbl: &[u8; TBL_SIZE], src: &[u8], len: usize) -> Result<String, GacsError> {
        let map_len: usize = (src.len() * 4).div_ceil(3);
        if len > map_len {
            return Err(GacsError::LenExceeded(len, map_len));
        }

        let mut mapped: Vec<u8> = Vec::with_capacity(map_len);
        for chunk in src.chunks(3) {
            match chunk {
                [b0, b1, b2] => {
                    mapped.push(tbl[(b0 >> 2) as usize]);
                    mapped.push(tbl[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
                    mapped.push(tbl[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
                    mapped.push(tbl[(b2 & 0x3f) as usize]);
                }
                [b0, b1] => {
                    mapped.push(tbl[(b0 >> 2) as usize]);
                    mapped.push(tbl[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize]);
                    mapped.push(tbl[((b1 & 0x0f) << 2) as usize]);
                }
                [b0] => {
                    mapped.push(tbl[(b0 >> 2) as usize]);
                    mapped.push(tbl[((b0 & 0x03) << 4) as usize]);
                }
                _ => unreachable!(),
            }
        }
        mapped.truncate(len);

        String::from_utf8(mapped).map_err(GacsError::InvalidOutput)
    }
}
