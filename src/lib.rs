const CHARSET_LEN: usize = 1 << 6;
const CHARSETS: [[u8; CHARSET_LEN]; 3] = [
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/", // BASE64
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_", // URL safe
    *b"ABCDEFGH!JKLMN@PQRSTUVWXYZabcdefghijk#mnopqrstuvwxyz$%23456789-_", // Password safe
];
const RULE_DELIMITER: char = ':';
const U32_BYTES: usize = ((u32::BITS) / (u8::BITS)) as usize;

const A: u64 = 1_664_525; // [alt1] 1103515245, [alt2] 214013, [alt3] 25214903917
const C: u64 = 1_013_904_223; // [alt1] 12345, [alt2] 2531011, [alt3] 11
const M: u64 = 1 << 32; // [alt1] 1 << 31, [alt2] 1 << 31, [alt3] 1 << 48

#[derive(Debug)]
pub enum GeneratorError {
    InvalidReplaceRule,
    InvalidReplaceChars,
    InvalidSource,
    SourceSliceError(std::array::TryFromSliceError),
    LengthExceedsMaximum(usize, usize),
    OutputUtf8ParseError(std::string::FromUtf8Error),
}

impl std::fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneratorError::InvalidReplaceRule => {
                write!(
                    f,
                    "invalid replace rule: expected 'target:replacement' (e.g., 'Zz9:^&*')"
                )
            }
            GeneratorError::InvalidReplaceChars => write!(
                f,
                "invalid replace characters: contains duplicates or characters not found in the base charset"
            ),
            GeneratorError::InvalidSource => write!(f, "invalid source data: length is too short"),
            GeneratorError::SourceSliceError(e) => write!(f, "failed to slice source: {e}"),
            GeneratorError::LengthExceedsMaximum(req, max) => {
                write!(
                    f,
                    "requested length ({req}) exceeds the maximum possible length ({max})"
                )
            }
            GeneratorError::OutputUtf8ParseError(e) => {
                write!(f, "failed to decode mapped output as UTF-8: {e}")
            }
        }
    }
}

impl std::error::Error for GeneratorError {}

pub enum Charset {
    Base64,
    UrlSafe,
    PasswordSafe,
}

impl Charset {
    fn charset(&self) -> &[u8; CHARSET_LEN] {
        match self {
            Self::Base64 => &CHARSETS[0],
            Self::UrlSafe => &CHARSETS[1],
            Self::PasswordSafe => &CHARSETS[2],
        }
    }
}

pub struct Generator {
    charset: [u8; CHARSET_LEN],
}

impl Generator {
    pub fn build(charset_e: &Charset, rule: Option<&String>) -> Result<Self, GeneratorError> {
        let charset: [u8; CHARSET_LEN] = match rule {
            Some(r) => {
                let (d, e) = r
                    .split_once(RULE_DELIMITER)
                    .ok_or(GeneratorError::InvalidReplaceRule)?;
                if !d.is_ascii() || !e.is_ascii() || d.len() != e.len() {
                    return Err(GeneratorError::InvalidReplaceRule);
                }
                charset_e
                    .charset()
                    .iter()
                    .copied()
                    .filter(|&c| !d.as_bytes().contains(&c))
                    .chain(e.bytes())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .map_err(|_| GeneratorError::InvalidReplaceChars)?
            }
            None => *charset_e.charset(),
        };

        Ok(Self { charset })
    }

    pub fn charset(&self) -> &[u8; CHARSET_LEN] {
        &self.charset
    }

    pub fn map(&self, src: &[u8], len: usize) -> Result<String, GeneratorError> {
        let (s_bytes, p_bytes) = src
            .split_at_checked(U32_BYTES)
            .ok_or(GeneratorError::InvalidSource)?;
        let scrambler: u32 = u32::from_be_bytes(
            s_bytes
                .try_into()
                .map_err(GeneratorError::SourceSliceError)?,
        );
        let map_len: usize = (p_bytes.len() * 4).div_ceil(3);
        if len > map_len {
            return Err(GeneratorError::LengthExceedsMaximum(len, map_len));
        }

        let s_charset: [u8; CHARSET_LEN] = self.scramble(scrambler);
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

        String::from_utf8(mapped).map_err(GeneratorError::OutputUtf8ParseError)
    }

    fn scramble(&self, scrambler: u32) -> [u8; CHARSET_LEN] {
        let mut scrambled: [u8; CHARSET_LEN] = self.charset;
        let mut s_rand: u64 = scrambler as u64;

        for i in 0..scrambled.len() {
            s_rand = (A * s_rand + C) % M;
            scrambled.swap(i, (s_rand as usize) % (i + 1));
        }

        scrambled
    }
}
