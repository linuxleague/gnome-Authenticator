use gettextrs::gettext;
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

impl Algorithm {
    pub fn to_locale_string(&self) -> String {
        match *self {
            Algorithm::HOTP => gettext("HMAC-based One-time Password"),
            Algorithm::TOTP => gettext("Time-based One-Time-Password"),
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
