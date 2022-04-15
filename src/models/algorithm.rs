use gettextrs::gettext;
use gtk::glib;
use ring::hmac;
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::{str::FromStr, string::ToString};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "OTPMethod")]
pub enum OTPMethod {
    #[enum_value(name = "TOTP")]
    TOTP = 0,
    #[enum_value(name = "HOTP")]
    HOTP = 1,
    Steam = 2,
}

impl Default for OTPMethod {
    fn default() -> Self {
        Self::TOTP
    }
}

impl Serialize for OTPMethod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for OTPMethod {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(OTPMethod::from_str(&String::deserialize(deserializer)?).unwrap())
    }
}

impl From<u32> for OTPMethod {
    fn from(u: u32) -> Self {
        match u {
            1 => OTPMethod::HOTP,
            2 => OTPMethod::Steam,
            _ => OTPMethod::default(),
        }
    }
}

impl OTPMethod {
    pub fn to_locale_string(self) -> String {
        match self {
            OTPMethod::HOTP => gettext("Counter-based"),
            OTPMethod::TOTP => gettext("Time-based"),
            // Translators: Steam refers to the gaming application by Valve.
            OTPMethod::Steam => gettext("Steam"),
        }
    }
}

impl FromStr for OTPMethod {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "totp" | "otp" => Ok(Self::TOTP),
            "hotp" => Ok(Self::HOTP),
            "steam" => Ok(Self::Steam),
            _ => anyhow::bail!("Unsupported OTPMethod"),
        }
    }
}

impl ToString for OTPMethod {
    fn to_string(&self) -> String {
        match *self {
            OTPMethod::TOTP => "totp",
            OTPMethod::HOTP => "hotp",
            OTPMethod::Steam => "steam",
        }
        .to_string()
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "Algorithm")]
pub enum Algorithm {
    #[enum_value(name = "SHA1")]
    SHA1 = 0,
    #[enum_value(name = "SHA256")]
    SHA256 = 1,
    #[enum_value(name = "SHA512")]
    SHA512 = 2,
}

impl Default for Algorithm {
    fn default() -> Self {
        Self::SHA1
    }
}

impl Serialize for Algorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Algorithm {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Algorithm::from_str(&String::deserialize(deserializer)?).unwrap())
    }
}

impl Algorithm {
    pub fn to_locale_string(self) -> String {
        match self {
            Algorithm::SHA1 => gettext("SHA-1"),
            Algorithm::SHA256 => gettext("SHA-256"),
            Algorithm::SHA512 => gettext("SHA-512"),
        }
    }
}

impl FromStr for Algorithm {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "sha1" => Ok(Self::SHA1),
            "sha256" => Ok(Self::SHA256),
            "sha512" => Ok(Self::SHA512),
            _ => anyhow::bail!("Unsupported HMAC-algorithm"),
        }
    }
}

impl ToString for Algorithm {
    fn to_string(&self) -> String {
        match *self {
            Algorithm::SHA1 => "sha1",
            Algorithm::SHA256 => "sha256",
            Algorithm::SHA512 => "sha512",
        }
        .to_string()
    }
}

impl From<Algorithm> for hmac::Algorithm {
    fn from(h: Algorithm) -> Self {
        match h {
            Algorithm::SHA1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            Algorithm::SHA256 => hmac::HMAC_SHA256,
            Algorithm::SHA512 => hmac::HMAC_SHA512,
        }
    }
}

impl From<u32> for Algorithm {
    fn from(u: u32) -> Self {
        match u {
            1 => Algorithm::SHA256,
            2 => Algorithm::SHA512,
            _ => Algorithm::default(),
        }
    }
}
