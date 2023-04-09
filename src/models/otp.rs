use std::{
    convert::TryInto,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use data_encoding::BASE32_NOPAD;
use ring::hmac;

use super::Algorithm;

pub static STEAM_CHARS: &str = "23456789BCDFGHJKMNPQRTVWXY";
pub static STEAM_DEFAULT_PERIOD: u32 = 30;
pub static STEAM_DEFAULT_DIGITS: u32 = 5;
pub static HOTP_DEFAULT_COUNTER: u32 = 1;
pub static DEFAULT_DIGITS: u32 = 6;
pub static TOTP_DEFAULT_PERIOD: u32 = 30;

/// Code graciously taken from the rust-otp crate.
/// <https://github.com/TimDumol/rust-otp/blob/master/src/lib.rs>

/// Decodes a secret (given as an RFC4648 base32-encoded ASCII string)
/// into a byte string. It fails if secret is not a valid Base32 string.
pub fn decode_secret(secret: &str) -> Result<Vec<u8>> {
    let secret = secret.trim().replace(' ', "").to_ascii_uppercase();
    // The buffer should have a length of secret.len() * 5 / 8.
    BASE32_NOPAD
        .decode(secret.as_bytes())
        .map_err(|_| anyhow!("Invalid Input"))
}

pub fn encode_secret(secret: &[u8]) -> String {
    BASE32_NOPAD.encode(secret)
}

/// Validates if `secret` is a valid Base32 String.
pub fn is_valid(secret: &str) -> bool {
    decode_secret(secret).is_ok()
}

/// Calculates the HMAC digest for the given secret and counter.
fn calc_digest(decoded_secret: &[u8], counter: u64, algorithm: Algorithm) -> hmac::Tag {
    let key = hmac::Key::new(algorithm.into(), decoded_secret);
    hmac::sign(&key, &counter.to_be_bytes())
}

/// Encodes the HMAC digest into a n-digit integer.
fn encode_digest(digest: &[u8]) -> Result<u32> {
    let offset = match digest.last() {
        Some(x) => *x & 0xf,
        None => anyhow::bail!("Invalid digest"),
    } as usize;
    let code_bytes: [u8; 4] = match digest[offset..offset + 4].try_into() {
        Ok(x) => x,
        Err(_) => anyhow::bail!("Invalid digest"),
    };
    let code = u32::from_be_bytes(code_bytes);
    Ok(code & 0x7fffffff)
}

/// Performs the [HMAC-based One-time Password Algorithm](http://en.wikipedia.org/wiki/HMAC-based_One-time_Password_Algorithm)
/// (HOTP) given an RFC4648 base32 encoded secret, and an integer counter.
pub(crate) fn hotp(secret: &[u8], counter: u64, algorithm: Algorithm, digits: u32) -> Result<u32> {
    let digest = encode_digest(calc_digest(secret, counter, algorithm).as_ref())?;
    Ok(digest % 10_u32.pow(digits))
}

pub(crate) fn steam(secret: &[u8], counter: u64) -> Result<String> {
    let mut full_token = encode_digest(calc_digest(secret, counter, Algorithm::SHA1).as_ref())?;

    let mut code = String::new();
    let total_chars = STEAM_CHARS.len() as u32;
    for _ in 0..STEAM_DEFAULT_DIGITS {
        let pos = full_token % total_chars;
        let charachter = STEAM_CHARS.chars().nth(pos as usize).unwrap();
        code.push(charachter);
        full_token /= total_chars;
    }
    Ok(code)
}

pub(crate) fn format(code: u32, digits: usize) -> String {
    let padded_code = format!("{code:0digits$}");
    let mut formated_code = String::new();
    for (idx, ch) in padded_code.chars().enumerate() {
        if (digits - idx) % 3 == 0 && (digits - idx) != 0 && idx != 0 {
            formated_code.push(' ');
        }
        formated_code.push(ch);
    }
    formated_code
}

pub(crate) fn time_based_counter(period: u32) -> u64 {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    timestamp / period as u64
}

#[cfg(test)]
mod tests {
    use super::{format, hotp, Algorithm, DEFAULT_DIGITS, TOTP_DEFAULT_PERIOD};
    use crate::models::Token;

    #[test]
    fn test_totp() {
        let secret_sha1 = b"12345678901234567890";
        let secret_sha256 = b"12345678901234567890123456789012";
        let secret_sha512 = b"1234567890123456789012345678901234567890123456789012345678901234";

        let counter1 = 59 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(94287082),
            hotp(secret_sha1, counter1, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(46119246),
            hotp(secret_sha256, counter1, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(90693936),
            hotp(secret_sha512, counter1, Algorithm::SHA512, 8).ok()
        );

        let counter2 = 1111111109 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(7081804),
            hotp(secret_sha1, counter2, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(68084774),
            hotp(secret_sha256, counter2, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(25091201),
            hotp(secret_sha512, counter2, Algorithm::SHA512, 8).ok()
        );

        let counter3 = 1111111111 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(14050471),
            hotp(secret_sha1, counter3, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(67062674),
            hotp(secret_sha256, counter3, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(99943326),
            hotp(secret_sha512, counter3, Algorithm::SHA512, 8).ok()
        );

        let counter4 = 1234567890 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(89005924),
            hotp(secret_sha1, counter4, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(91819424),
            hotp(secret_sha256, counter4, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(93441116),
            hotp(secret_sha512, counter4, Algorithm::SHA512, 8).ok()
        );

        let counter5 = 2000000000 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(69279037),
            hotp(secret_sha1, counter5, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(90698825),
            hotp(secret_sha256, counter5, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(38618901),
            hotp(secret_sha512, counter5, Algorithm::SHA512, 8).ok()
        );

        let counter6 = 20000000000 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(65353130),
            hotp(secret_sha1, counter6, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(77737706),
            hotp(secret_sha256, counter6, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(47863826),
            hotp(secret_sha512, counter6, Algorithm::SHA512, 8).ok()
        );
    }

    // Some of the tests are heavily inspired(copy-paste) of the andOTP application
    #[test]
    fn test_hotp() {
        let token = Token::from_str("BASE32SECRET3232", Algorithm::SHA1, DEFAULT_DIGITS).unwrap();
        assert_eq!(token.hotp(0).ok(), Some(260182));
        assert_eq!(token.hotp(1).ok(), Some(55283));
        assert_eq!(token.hotp(1401).ok(), Some(316439));

        let token = Token::from_bytes(b"12345678901234567890", Algorithm::SHA1, DEFAULT_DIGITS);
        assert_eq!(Some(755224), token.hotp(0).ok(),);
        assert_eq!(Some(287082), token.hotp(1).ok());
        assert_eq!(Some(359152), token.hotp(2).ok());
        assert_eq!(Some(969429), token.hotp(3).ok());
        assert_eq!(Some(338314), token.hotp(4).ok());
        assert_eq!(Some(254676), token.hotp(5).ok());
        assert_eq!(Some(287922), token.hotp(6).ok());
        assert_eq!(Some(162583), token.hotp(7).ok());
        assert_eq!(Some(399871), token.hotp(8).ok());
        assert_eq!(Some(520489), token.hotp(9).ok());
    }

    #[test]
    fn test_steam() {
        let token = Token::from_str_steam("BASE32SECRET3232").unwrap();
        assert_eq!(token.steam(0).ok(), Some("2TC8B".into()));
        assert_eq!(token.steam(1).ok(), Some("YKKK4".into()));
    }

    #[test]
    fn otp_format() {
        assert_eq!(format(1234, 5), "01 234");
        assert_eq!(format(1234, 6), "001 234");
        assert_eq!(format(123456, 6), "123 456");
        assert_eq!(format(1234, 7), "0 001 234");
        assert_eq!(format(1234567, 8), "01 234 567");
        assert_eq!(format(12345678, 8), "12 345 678");
    }
}
