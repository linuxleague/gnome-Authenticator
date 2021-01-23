use super::Algorithm;
use anyhow::Result;
use data_encoding::BASE32_NOPAD;
use ring::hmac;
use std::convert::TryInto;

static STEAM_CHARS: &str = "23456789BCDFGHJKMNPQRTVWXY";
static STEAM_DEFAULT_COUNTER: u64 = 30;
static STEAM_DEFAULT_DIGITS: u32 = 5;

/// Code graciously taken from the rust-top crate.
/// https://github.com/TimDumol/rust-otp/blob/master/src/lib.rs

/// Decodes a secret (given as an RFC4648 base32-encoded ASCII string)
/// into a byte string
fn decode_secret(secret: &str) -> Result<Vec<u8>> {
    let res = BASE32_NOPAD.decode(secret.as_bytes())?;
    Ok(res)
}

/// Calculates the HMAC digest for the given secret and counter.
fn calc_digest(decoded_secret: &[u8], counter: u64, algorithm: hmac::Algorithm) -> hmac::Tag {
    let key = hmac::Key::new(algorithm, decoded_secret);
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
pub(crate) fn hotp(
    secret: &str,
    counter: u64,
    algorithm: hmac::Algorithm,
    digits: u32,
) -> Result<u32> {
    let decoded = decode_secret(secret)?;
    let digest = encode_digest(calc_digest(decoded.as_slice(), counter, algorithm).as_ref())?;
    Ok(digest % 10_u32.pow(digits))
}

pub(crate) fn steam(secret: &str) -> Result<String> {
    let mut token = hotp(
        secret,
        STEAM_DEFAULT_COUNTER,
        Algorithm::SHA1.into(),
        STEAM_DEFAULT_DIGITS,
    )?;
    let mut code = String::new();
    let total_chars = STEAM_CHARS.len() as u32;
    for _ in 0..5 {
        let pos = token % total_chars;
        let charachter = STEAM_CHARS.chars().nth(pos as usize).unwrap();
        code.push(charachter);
        token = token / total_chars;
    }
    Ok(code)
}

pub(crate) fn format(code: u32, digits: usize) -> String {
    let mut formated_code = format!("{:0width$}", code, width = digits);
    if digits % 2 == 0 {
        let middle = digits / 2;
        formated_code.insert(middle, ' ');
    }
    formated_code
}

#[cfg(test)]
mod tests {
    use super::{format, hmac, hotp};

    #[test]
    fn test_htop() {
        let algorithm = hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY;
        let digits: u32 = 6;
        assert_eq!(
            hotp("BASE32SECRET3232", 0, algorithm, digits).unwrap(),
            260182
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1, algorithm, digits).unwrap(),
            55283
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1401, algorithm, digits).unwrap(),
            316439
        );
    }

    #[test]
    fn test_otp_format() {
        assert_eq!(format(012345, 6), "012 345");
        assert_eq!(format(01234, 5), "01234");
    }
}
