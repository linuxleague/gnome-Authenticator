use crate::models::{Account, Algorithm, OTPMethod};
use percent_encoding::percent_decode_str;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct OTPUri {
    pub algorithm: Algorithm,
    pub label: String,
    pub secret: String,
    pub issuer: String,
    pub method: OTPMethod,
    pub digits: Option<i32>,
    pub period: Option<i32>,
    pub counter: Option<i32>,
}

impl FromStr for OTPUri {
    type Err = anyhow::Error;
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let url = url::Url::parse(uri)?;

        if url.scheme() != "otpauth" {
            anyhow::bail!("Invalid OTP uri format, expected otpauth");
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
            account_info.get(0).unwrap()
        } else {
            // If we have "Provider:Account"
            provider_name = Some(account_info.get(0).unwrap().to_string());
            account_info.get(1).unwrap()
        };

        pairs.for_each(|(key, value)| match key.into_owned().as_str() {
            "period" => {
                period = value.parse::<i32>().ok();
            }
            "digits" => {
                digits = value.parse::<i32>().ok();
            }
            "counter" => {
                counter = value.parse::<i32>().ok();
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

impl Into<String> for OTPUri {
    fn into(self) -> String {
        format!(
            "otpauth://{}/{}?secret={}&issuer={}&algorithm={}&digits={}&counter={}",
            self.method.to_string(),
            self.label,
            self.secret,
            self.issuer,
            self.algorithm.to_string(),
            self.digits.unwrap_or(6),
            self.counter.unwrap_or(1),
        )
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