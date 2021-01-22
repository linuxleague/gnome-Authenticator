use anyhow::Result;
use data_encoding::{DecodeError, BASE32_NOPAD};
use ring::hmac;
use std::convert::TryInto;
use std::time::{SystemTime, SystemTimeError};

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
fn encode_digest(digest: &[u8], digits: u32) -> Result<u32> {
    let offset = match digest.last() {
        Some(x) => *x & 0xf,
        None => anyhow::bail!("Invalid digest"),
    } as usize;
    let code_bytes: [u8; 4] = match digest[offset..offset + 4].try_into() {
        Ok(x) => x,
        Err(_) => anyhow::bail!("Invalid digest"),
    };
    let code = u32::from_be_bytes(code_bytes);
    Ok((code & 0x7fffffff) % 10_u32.pow(digits))
}

/// Performs the [HMAC-based One-time Password Algorithm](http://en.wikipedia.org/wiki/HMAC-based_One-time_Password_Algorithm)
/// (HOTP) given an RFC4648 base32 encoded secret, and an integer counter.
pub(crate) fn generate_hotp(
    secret: &str,
    counter: u64,
    algorithm: hmac::Algorithm,
    digits: u32,
) -> Result<u32> {
    let decoded = decode_secret(secret)?;
    encode_digest(
        calc_digest(decoded.as_slice(), counter, algorithm).as_ref(),
        digits,
    )
}

#[cfg(test)]
mod tests {
    use super::make_hotp;

    #[test]
    fn hotp() {
        let algorithm = hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY;
        let digits: u32 = 6;
        assert_eq!(
            make_hotp("BASE32SECRET3232", 0, algorithm, digits).unwrap(),
            260182
        );
        assert_eq!(
            make_hotp("BASE32SECRET3232", 1, algorithm, digits).unwrap(),
            55283
        );
        assert_eq!(
            make_hotp("BASE32SECRET3232", 1401, algorithm, digits).unwrap(),
            316439
        );
    }
}
