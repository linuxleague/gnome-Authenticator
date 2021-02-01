use super::Algorithm;
use anyhow::Result;
use ring::hmac;
use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};

pub static STEAM_CHARS: &str = "23456789BCDFGHJKMNPQRTVWXY";
pub static STEAM_DEFAULT_PERIOD: u32 = 30;
pub static STEAM_DEFAULT_DIGITS: u32 = 5;
pub static HOTP_DEFAULT_COUNTER: u32 = 1;
pub static DEFAULT_DIGITS: u32 = 6;
pub static TOTP_DEFAULT_PERIOD: u32 = 30;

/// Code graciously taken from the rust-top crate.
/// https://github.com/TimDumol/rust-otp/blob/master/src/lib.rs

/// Decodes a secret (given as an RFC4648 base32-encoded ASCII string)
/// into a byte string. It fails if secret is not a valid Base32 string.
fn decode_secret(secret: &str) -> Result<Vec<u8>> {
    let secret = secret.trim().replace(' ', "").to_uppercase();

    data_encoding::BASE32_NOPAD
        .decode(secret.as_bytes())
        .map_err(From::from)
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
pub(crate) fn hotp(secret: &str, counter: u64, algorithm: Algorithm, digits: u32) -> Result<u32> {
    let decoded = decode_secret(secret)?;
    let digest = encode_digest(calc_digest(decoded.as_slice(), counter, algorithm).as_ref())?;
    Ok(digest % 10_u32.pow(digits))
}

pub(crate) fn steam(secret: &str, counter: u64) -> Result<String> {
    let decoded = decode_secret(secret)?;
    let mut full_token =
        encode_digest(calc_digest(decoded.as_slice(), counter, Algorithm::SHA1).as_ref())?;

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
    let padded_code = format!("{:0width$}", code, width = digits);
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
    use super::{format, hotp, steam, Algorithm, DEFAULT_DIGITS};

    #[test]
    fn test_hotp() {
        assert_eq!(
            hotp("BASE32SECRET3232", 0, Algorithm::SHA1, DEFAULT_DIGITS).unwrap(),
            260182
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1, Algorithm::SHA1, DEFAULT_DIGITS).unwrap(),
            55283
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1401, Algorithm::SHA1, DEFAULT_DIGITS).unwrap(),
            316439
        );
    }

    #[test]
    fn test_steam_totp() {
        assert_eq!(steam("BASE32SECRET3232", 0).unwrap(), "2TC8B");
        assert_eq!(steam("BASE32SECRET3232", 1).unwrap(), "YKKK4");
    }

    #[test]
    fn test_otp_format() {
        assert_eq!(format(01234, 5), "01 234");
        assert_eq!(format(01234, 6), "001 234");
        assert_eq!(format(123456, 6), "123 456");
        assert_eq!(format(01234, 7), "0 001 234");
        assert_eq!(format(01234567, 8), "01 234 567");
        assert_eq!(format(12345678, 8), "12 345 678");
    }
}
