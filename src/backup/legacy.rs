use anyhow::Result;
use gettextrs::gettext;
use serde::Deserialize;

use super::{Restorable, RestorableItem};
use crate::models::{Algorithm, Method};

// Same as andOTP except uses the first tag for the issuer
#[derive(Deserialize)]
pub struct LegacyAuthenticator {
    pub secret: String,
    pub label: String,
    pub digits: u32,
    #[serde(rename = "type")]
    pub method: Method,
    pub algorithm: Algorithm,
    pub thumbnail: String,
    pub last_used: i64,
    pub tags: Vec<String>,
    pub period: u32,
}

impl Restorable for LegacyAuthenticator {
    const ENCRYPTABLE: bool = false;
    const SCANNABLE: bool = false;

    type Item = Self;

    fn identifier() -> String {
        "authenticator_legacy".to_string()
    }

    fn title() -> String {
        // Translators: this is for restoring a backup from the old Authenticator
        // release
        gettext("Au_thenticator (Legacy)")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore_from_data(from: &[u8], _key: Option<&str>) -> Result<Vec<Self::Item>> {
        serde_json::de::from_slice(from).map_err(From::from)
    }
}

impl RestorableItem for LegacyAuthenticator {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.tags
            .get(0)
            .cloned()
            .unwrap_or_else(|| "Default".to_string())
    }

    fn secret(&self) -> String {
        self.secret.clone()
    }

    fn period(&self) -> Option<u32> {
        Some(self.period)
    }

    fn method(&self) -> Method {
        self.method
    }

    fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    fn digits(&self) -> Option<u32> {
        Some(self.digits)
    }

    fn counter(&self) -> Option<u32> {
        None
    }
}
