use std::{fmt::Write, str::FromStr};

use percent_encoding::percent_decode_str;
use url::Url;

use crate::{
    backup::RestorableItem,
    models::{otp, Account, Algorithm, OTPMethod},
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub struct OTPUri {
    pub algorithm: Algorithm,
    pub label: String,
    pub secret: String,
    pub issuer: String,
    pub method: OTPMethod,
    pub digits: Option<u32>,
    pub period: Option<u32>,
    pub counter: Option<u32>,
}

impl RestorableItem for OTPUri {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn secret(&self) -> String {
        self.secret.clone()
    }

    fn period(&self) -> Option<u32> {
        self.period
    }

    fn method(&self) -> OTPMethod {
        self.method
    }

    fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    fn digits(&self) -> Option<u32> {
        self.digits
    }

    fn counter(&self) -> Option<u32> {
        self.counter
    }
}

impl TryFrom<Url> for OTPUri {
    type Error = anyhow::Error;
    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.scheme() != "otpauth" {
            anyhow::bail!(
                "Invalid OTP uri format, expected otpauth, got {}",
                url.scheme()
            );
        }

        let mut period = None;
        let mut counter = None;
        let mut digits = None;
        let mut provider_name = None;
        let mut algorithm = None;
        let mut secret = None;

        let pairs = url.query_pairs();

        let method = OTPMethod::from_str(url.host_str().unwrap())?;

        let account_info = url
            .path()
            .trim_start_matches('/')
            .split(':')
            .collect::<Vec<&str>>();

        let account_name = if account_info.len() == 1 {
            account_info.first().unwrap()
        } else {
            // If we have "Provider:Account"
            provider_name = Some(account_info.first().unwrap().to_string());
            account_info.get(1).unwrap()
        };

        pairs.for_each(|(key, value)| match key.into_owned().as_str() {
            "period" => {
                period = value.parse::<u32>().ok();
            }
            "digits" => {
                digits = value.parse::<u32>().ok();
            }
            "counter" => {
                counter = value.parse::<u32>().ok();
            }
            "issuer" => {
                provider_name = Some(value.to_string());
            }
            "algorithm" => {
                algorithm = Algorithm::from_str(&value).ok();
            }
            "secret" => {
                secret = Some(value.to_string());
            }
            _ => (),
        });

        if secret.is_none() {
            anyhow::bail!("OTP uri must contain a secret");
        }

        let label = percent_decode_str(account_name).decode_utf8()?.into_owned();
        let issuer = if let Some(n) = provider_name {
            percent_decode_str(&n).decode_utf8()?.into_owned()
        } else {
            "Default".to_string()
        };

        Ok(Self {
            method,
            label,
            secret: secret.unwrap(),
            issuer,
            algorithm: algorithm.unwrap_or_default(),
            digits,
            period,
            counter,
        })
    }
}

impl FromStr for OTPUri {
    type Err = anyhow::Error;
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(uri)?;
        OTPUri::try_from(url)
    }
}

impl From<OTPUri> for String {
    fn from(val: OTPUri) -> Self {
        let mut otp_uri = format!(
            "otpauth://{}/{}?secret={}&issuer={}&algorithm={}",
            val.method.to_string(),
            val.label,
            val.secret,
            val.issuer,
            val.algorithm.to_string(),
        );
        if let Some(digits) = val.digits {
            write!(otp_uri, "&digits={digits}").unwrap();
        }
        if val.method == OTPMethod::HOTP {
            write!(
                otp_uri,
                "&counter={}",
                val.counter.unwrap_or(otp::HOTP_DEFAULT_COUNTER)
            )
            .unwrap();
        } else {
            write!(
                otp_uri,
                "&period={}",
                val.period.unwrap_or(otp::TOTP_DEFAULT_PERIOD)
            )
            .unwrap();
        }
        otp_uri
    }
}

impl From<&Account> for OTPUri {
    fn from(a: &Account) -> Self {
        Self {
            method: a.provider().method(),
            label: a.name(),
            secret: a.token(),
            issuer: a.provider().name(),
            algorithm: a.provider().algorithm(),
            digits: Some(a.provider().digits()),
            period: Some(a.provider().period()),
            counter: Some(a.counter()),
        }
    }
}
