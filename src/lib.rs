use std::{fs::File, io::Read, path::Path};

use sha2::{Digest, Sha512, digest::Output};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GacsError {
    #[error("Invalid rule: expected 'remove:add' (e.g., 'Zz9:^&*')")]
    InvalidRuleFmt,

    #[error("Invalid rule: contains duplicates or characters not found in the charset")]
    InvalidRuleChars,

    #[error("Source is too short")]
    ShortSrc,

    #[error("Requested length ({0}) exceeds the maximum length ({1})")]
    LengthExceeded(usize, usize),

    #[error("Failed to slice source: {0}")]
    SliceSrc(std::array::TryFromSliceError),

    #[error("File operation failed: {0}")]
    File(std::io::Error),

    #[error("Generated string contains invalid UTF-8: {0}")]
    InvalidOutput(std::string::FromUtf8Error),
}

pub enum Charset {
    Base64,
    UrlSafe,
    PasswordSafe,
    ShellSafe,
}

impl Charset {
    const TBL_SIZE: usize = 1 << 6;
    const BASE_TBLS: [[u8; Self::TBL_SIZE]; 4] = [
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", // BASE64
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_", // URL-Safe
        *b"ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_", // Password-Safe
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._", // Shell-Safe
    ];

    fn tbl(&self) -> &[u8; Self::TBL_SIZE] {
        match self {
            Self::Base64 => &Self::BASE_TBLS[0],
            Self::UrlSafe => &Self::BASE_TBLS[1],
            Self::PasswordSafe => &Self::BASE_TBLS[2],
            Self::ShellSafe => &Self::BASE_TBLS[3],
        }
    }
}

pub struct Gacs {
    tbl: [u8; Charset::TBL_SIZE],
}

impl Gacs {
    const A: u64 = 1_664_525; // [alt1] 1103515245, [alt2] 214013, [alt3] 25214903917
    const C: u64 = 1_013_904_223; // [alt1] 12345, [alt2] 2531011, [alt3] 11
    const M: u64 = 1 << 32; // [alt1] 1 << 31, [alt2] 1 << 31, [alt3] 1 << 48

    const FILE_BUFFER_SIZE: usize = 8_192;
    const SRC_SIZE: usize = std::mem::size_of::<Output<Sha512>>();
    const RULE_DELIM: char = ':';
    const SALT_DELIM: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

    pub fn build(charset: &Charset, rule: Option<&str>) -> Result<Self, GacsError> {
        let tbl: [u8; Charset::TBL_SIZE] = match rule {
            Some(r) => {
                let (rm, ad) = r
                    .split_once(Self::RULE_DELIM)
                    .ok_or(GacsError::InvalidRuleFmt)?;
                if (!rm.is_ascii()) || (!ad.is_ascii()) || (rm.len() != ad.len()) {
                    return Err(GacsError::InvalidRuleFmt);
                }
                charset
                    .tbl()
                    .iter()
                    .copied()
                    .filter(|&c| !rm.as_bytes().contains(&c))
                    .chain(ad.bytes())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .map_err(|_| GacsError::InvalidRuleChars)?
            }
            None => *charset.tbl(),
        };

        Ok(Self { tbl })
    }

    pub fn tbl(&self) -> &[u8; Charset::TBL_SIZE] {
        &self.tbl
    }

    pub fn generate(
        &self,
        seed: &str,
        salt: Option<&Path>,
        length: Option<usize>,
    ) -> Result<String, GacsError> {
        let src: [u8; Self::SRC_SIZE] = self.src(seed, salt)?;
        let (s_src, c_src) = src
            .split_at_checked(std::mem::size_of::<u32>())
            .ok_or(GacsError::ShortSrc)?;

        let shuffler: u32 = u32::from_be_bytes(s_src.try_into().map_err(GacsError::SliceSrc)?);
        let s_tbl: [u8; Charset::TBL_SIZE] = self.shuffle(shuffler);

        self.map(&s_tbl, c_src, length)
    }

    fn src(&self, seed: &str, salt: Option<&Path>) -> Result<[u8; Self::SRC_SIZE], GacsError> {
        let mut hasher: Sha512 = Sha512::new();

        hasher.update(seed.as_bytes());

        if let Some(p) = salt {
            hasher.update(Self::SALT_DELIM);

            let mut file: File = File::open(p).map_err(GacsError::File)?;
            let mut buf: [u8; Self::FILE_BUFFER_SIZE] = [0u8; Self::FILE_BUFFER_SIZE];
            loop {
                let cnt: usize = file.read(&mut buf).map_err(GacsError::File)?;
                if cnt == 0 {
                    break;
                }
                hasher.update(&buf[..cnt]);
            }
        }

        Ok(hasher.finalize().into())
    }

    fn shuffle(&self, shuffler: u32) -> [u8; Charset::TBL_SIZE] {
        let mut shuffled: [u8; Charset::TBL_SIZE] = self.tbl;
        let mut s_rand: u64 = shuffler as u64;

        for i in 0..shuffled.len() {
            s_rand = (Self::A * s_rand + Self::C) % Self::M;
            shuffled.swap(i, (s_rand as usize) % (i + 1));
        }

        shuffled
    }

    fn map(
        &self,
        tbl: &[u8; Charset::TBL_SIZE],
        src: &[u8],
        length: Option<usize>,
    ) -> Result<String, GacsError> {
        let map_len: usize = (src.len() * 4).div_ceil(3);
        let len: usize = match length {
            Some(l) => {
                if l > map_len {
                    return Err(GacsError::LengthExceeded(l, map_len));
                } else {
                    l
                }
            }
            None => map_len,
        };

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
