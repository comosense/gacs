use std::time::{SystemTime, UNIX_EPOCH};

use gacs::{Charset, Gacs, GacsError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SeedError {
    #[error("Gacs error: {0}")]
    Gacs(#[from] GacsError),

    #[error("Failed to get System Time: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

pub enum SeedFrom {
    Cli,
    Auto,
}

pub struct Seed {
    seed: String,
    from: SeedFrom,
}

impl Seed {
    pub fn new(seed: Option<&str>) -> Self {
        match seed {
            Some(s) => Self {
                seed: s.to_string(),
                from: SeedFrom::Cli,
            },
            None => Self {
                seed: String::new(),
                from: SeedFrom::Auto,
            },
        }
    }

    pub fn update(&mut self) -> Result<(), SeedError> {
        if let SeedFrom::Auto = self.from {
            let sb: String = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_nanos()
                .to_string()
                + &self.seed;
            self.seed = Gacs::build(&Charset::ShellSafe, None)?.generate(&sb, None, None)?;
        }
        Ok(())
    }

    pub fn seed(&self) -> &str {
        &self.seed
    }

    pub fn from(&self) -> &SeedFrom {
        &self.from
    }
}
