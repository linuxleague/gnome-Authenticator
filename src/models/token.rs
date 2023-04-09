use anyhow::Result;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::{
    otp::{self, STEAM_DEFAULT_DIGITS},
    Algorithm,
};

#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct Token {
    secret: Vec<u8>,
    #[zeroize(skip)]
    algorithm: Algorithm,
    #[zeroize(skip)]
    digits: u32,
}

impl Token {
    pub fn from_bytes_steam(secret: &[u8]) -> Self {
        Self::from_bytes(secret, Algorithm::SHA1, STEAM_DEFAULT_DIGITS)
    }

    pub fn from_str_steam(secret: &str) -> Result<Self> {
        Self::from_str(secret, Algorithm::SHA1, STEAM_DEFAULT_DIGITS)
    }

    pub fn from_str(secret: &str, algorithm: Algorithm, digits: u32) -> Result<Self> {
        let decoded = otp::decode_secret(secret)?;
        Ok(Self::from_bytes(&decoded, algorithm, digits))
    }

    pub fn from_bytes(secret: &[u8], algorithm: Algorithm, digits: u32) -> Self {
        Self {
            secret: secret.to_owned(),
            algorithm,
            digits,
        }
    }

    pub fn hotp(&self, counter: u64) -> Result<u32> {
        otp::hotp(&self.secret, counter, self.algorithm, self.digits)
    }

    pub fn hotp_formatted(&self, counter: u64) -> Result<String> {
        self.hotp(counter)
            .map(|d| otp::format(d, self.digits as usize))
    }

    pub fn steam(&self, counter: u64) -> Result<String> {
        otp::steam(&self.secret, counter)
    }

    pub fn as_string(&self) -> String {
        otp::encode_secret(&self.secret)
    }
}
