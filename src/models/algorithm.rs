use gettextrs::gettext;
use ring::hmac;
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};
use std::str::FromStr;
use std::string::ToString;

#[derive(Debug, Eq, PartialEq, Clone, Copy, GEnum)]
#[repr(u32)]
#[genum(type_name = "ProviderAlgorithm")]
pub enum Algorithm {
    #[genum(name = "TOTP")]
    TOTP = 0,
    #[genum(name = "HOTP")]
    HOTP = 1,
    Steam = 2,
}

impl Default for Algorithm {
    fn default() -> Self {
        Self::TOTP
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

impl From<u32> for Algorithm {
    fn from(u: u32) -> Self {
        match u {
            1 => Algorithm::HOTP,
            2 => Algorithm::Steam,
            _ => Algorithm::default(),
        }
    }
}

impl Algorithm {
    pub fn to_locale_string(&self) -> String {
        match *self {
            Algorithm::HOTP => gettext("HMAC-based"),
            Algorithm::TOTP => gettext("Time-based"),
            Algorithm::Steam => gettext("Steam"),
        }
    }
}

impl FromStr for Algorithm {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "totp" | "otp" => Ok(Self::TOTP),
            "hotp" => Ok(Self::HOTP),
            "steam" => Ok(Self::Steam),
            _ => anyhow::bail!("Unsupported algorithm"),
        }
    }
}

impl ToString for Algorithm {
    fn to_string(&self) -> String {
        match *self {
            Algorithm::TOTP => "totp",
            Algorithm::HOTP => "hotp",
            Algorithm::Steam => "steam",
        }
        .to_string()
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, GEnum)]
#[repr(u32)]
#[genum(type_name = "ProviderHOTPAlgorithm")]
pub enum HOTPAlgorithm {
    #[genum(name = "SHA1")]
    SHA1 = 0,
    #[genum(name = "SHA256")]
    SHA256 = 1,
    #[genum(name = "SHA512")]
    SHA512 = 2,
}

impl Default for HOTPAlgorithm {
    fn default() -> Self {
        Self::SHA1
    }
}

impl Serialize for HOTPAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HOTPAlgorithm {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(HOTPAlgorithm::from_str(&String::deserialize(deserializer)?).unwrap())
    }
}

impl HOTPAlgorithm {
    pub fn to_locale_string(&self) -> String {
        match *self {
            HOTPAlgorithm::SHA1 => gettext("SHA1"),
            HOTPAlgorithm::SHA256 => gettext("SHA256"),
            HOTPAlgorithm::SHA512 => gettext("SHA512"),
        }
    }
}

impl FromStr for HOTPAlgorithm {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "sha1" | "otp" => Ok(Self::SHA1),
            "sha256" => Ok(Self::SHA256),
            "sha512" => Ok(Self::SHA512),
            _ => anyhow::bail!("Unsupported HMAC algorithm"),
        }
    }
}

impl ToString for HOTPAlgorithm {
    fn to_string(&self) -> String {
        match *self {
            HOTPAlgorithm::SHA1 => "sha1",
            HOTPAlgorithm::SHA256 => "sha256",
            HOTPAlgorithm::SHA512 => "sha512",
        }
        .to_string()
    }
}

impl From<HOTPAlgorithm> for hmac::Algorithm {
    fn from(h: HOTPAlgorithm) -> Self {
        match h {
            HOTPAlgorithm::SHA1 => hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            HOTPAlgorithm::SHA256 => hmac::HMAC_SHA256,
            HOTPAlgorithm::SHA512 => hmac::HMAC_SHA512,
        }
    }
}
