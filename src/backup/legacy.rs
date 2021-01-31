use super::{Restorable, RestorableItem};
use crate::models::{Algorithm, OTPMethod};
use anyhow::Result;
use gettextrs::gettext;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};

// Same as andOTP except uses the first tag for the issuer
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyAuthenticator {
    pub secret: String,
    pub label: String,
    pub digits: u32,
    #[serde(rename = "type")]
    pub method: OTPMethod,
    pub algorithm: Algorithm,
    pub thumbnail: String,
    pub last_used: i64,
    pub tags: Vec<String>,
    pub period: u32,
}

impl Restorable for LegacyAuthenticator {
    type Item = Self;

    fn identifier() -> String {
        "authenticator_legacy".to_string()
    }

    fn title() -> String {
        // Translators: this is for restoring a backup from the old Authenticator release
        gettext("Au_thenticator (Legacy)")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::NONE_CANCELLABLE)?;
        let items: Vec<LegacyAuthenticator> = serde_json::de::from_slice(&data)?;
        Ok(items)
    }
}

impl RestorableItem for LegacyAuthenticator {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.tags
            .get(0)
            .map(|s| s.clone())
            .unwrap_or_else(|| "Default".to_string())
    }

    fn secret(&self) -> String {
        self.secret.clone()
    }

    fn period(&self) -> Option<u32> {
        Some(self.period)
    }

    fn method(&self) -> OTPMethod {
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
