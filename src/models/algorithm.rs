use glib::StaticType;
use std::str::FromStr;
use std::string::ToString;

#[derive(Debug, Eq, PartialEq, Clone, Copy, GEnum)]
#[repr(u32)]
#[genum(type_name = "ProviderAlgorithm")]
pub enum Algorithm {
    #[genum(name = "OTP")]
    OTP = 0,
    #[genum(name = "HOTP")]
    HOTP = 1,
    #[genum(name = "Steam")]
    Steam = 2,
}

impl FromStr for Algorithm {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "otp" => Ok(Self::OTP),
            "hotp" => Ok(Self::HOTP),
            "steam" => Ok(Self::Steam),
            _ => anyhow::bail!("Unsupported algorithm"),
        }
    }
}

impl ToString for Algorithm {
    fn to_string(&self) -> String {
        match *self {
            Algorithm::OTP => "otp",
            Algorithm::HOTP => "hotp",
            Algorithm::Steam => "steam",
        }
        .to_string()
    }
}
