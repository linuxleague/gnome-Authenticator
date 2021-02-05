use super::Algorithm;
use anyhow::{anyhow, Result};
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
    // The buffer should have a length of secret.len() * 5 / 8.
    let size = secret.len();
    let mut output_buffer = std::iter::repeat(0).take(size).collect::<Vec<u8>>();
    let vec = binascii::b32decode(secret.as_bytes(), &mut output_buffer)
        .map_err(|_| anyhow!("Invalid Input"))?
        .to_vec();

    Ok(vec)
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
    use super::{format, hotp, steam, Algorithm, DEFAULT_DIGITS, TOTP_DEFAULT_PERIOD};
    #[test]
    fn test_totp() {
        let secret_sha1 = String::from_utf8(
            binascii::b32encode(b"12345678901234567890", &mut [0; 64])
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        let secret_sha256 = String::from_utf8(
            binascii::b32encode(b"12345678901234567890123456789012", &mut [0; 64])
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        let secret_sha512 = String::from_utf8(
            binascii::b32encode(
                b"1234567890123456789012345678901234567890123456789012345678901234",
                &mut [0; 128],
            )
            .unwrap()
            .to_vec(),
        )
        .unwrap();

        let counter1 = 59 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(94287082),
            hotp(&secret_sha1, counter1, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(46119246),
            hotp(&secret_sha256, counter1, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(90693936),
            hotp(&secret_sha512, counter1, Algorithm::SHA512, 8).ok()
        );

        let counter2 = 1111111109 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(7081804),
            hotp(&secret_sha1, counter2, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(68084774),
            hotp(&secret_sha256, counter2, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(25091201),
            hotp(&secret_sha512, counter2, Algorithm::SHA512, 8).ok()
        );

        let counter3 = 1111111111 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(14050471),
            hotp(&secret_sha1, counter3, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(67062674),
            hotp(&secret_sha256, counter3, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(99943326),
            hotp(&secret_sha512, counter3, Algorithm::SHA512, 8).ok()
        );

        let counter4 = 1234567890 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(89005924),
            hotp(&secret_sha1, counter4, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(91819424),
            hotp(&secret_sha256, counter4, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(93441116),
            hotp(&secret_sha512, counter4, Algorithm::SHA512, 8).ok()
        );

        let counter5 = 2000000000 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(69279037),
            hotp(&secret_sha1, counter5, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(90698825),
            hotp(&secret_sha256, counter5, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(38618901),
            hotp(&secret_sha512, counter5, Algorithm::SHA512, 8).ok()
        );

        let counter6 = 20000000000 / TOTP_DEFAULT_PERIOD as u64;
        assert_eq!(
            Some(65353130),
            hotp(&secret_sha1, counter6, Algorithm::SHA1, 8).ok()
        );
        assert_eq!(
            Some(77737706),
            hotp(&secret_sha256, counter6, Algorithm::SHA256, 8).ok()
        );
        assert_eq!(
            Some(47863826),
            hotp(&secret_sha512, counter6, Algorithm::SHA512, 8).ok()
        );
    }

    // Some of the tests are heavily inspired(copy-paste) of the andOTP application
    #[test]
    fn test_hotp() {
        assert_eq!(
            hotp("BASE32SECRET3232", 0, Algorithm::SHA1, DEFAULT_DIGITS).ok(),
            Some(260182)
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1, Algorithm::SHA1, DEFAULT_DIGITS).ok(),
            Some(55283)
        );
        assert_eq!(
            hotp("BASE32SECRET3232", 1401, Algorithm::SHA1, DEFAULT_DIGITS).ok(),
            Some(316439)
        );
        let secret = String::from_utf8(
            binascii::b32encode(b"12345678901234567890", &mut [0; 64])
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(
            Some(755224),
            hotp(&secret, 0, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(287082),
            hotp(&secret, 1, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(359152),
            hotp(&secret, 2, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(969429),
            hotp(&secret, 3, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(338314),
            hotp(&secret, 4, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(254676),
            hotp(&secret, 5, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(287922),
            hotp(&secret, 6, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(162583),
            hotp(&secret, 7, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(399871),
            hotp(&secret, 8, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
        assert_eq!(
            Some(520489),
            hotp(&secret, 9, Algorithm::SHA1, DEFAULT_DIGITS).ok()
        );
    }

    #[test]
    fn test_steam_totp() {
        assert_eq!(steam("BASE32SECRET3232", 0).ok(), Some("2TC8B".into()));
        assert_eq!(steam("BASE32SECRET3232", 1).ok(), Some("YKKK4".into()));
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
