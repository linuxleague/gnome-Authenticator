use super::Restorable;
use crate::models::{otp, Account, Algorithm, OTPMethod, ProvidersModel};
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
        gettext("Au_thenticator (Old)")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::NONE_CANCELLABLE)?;
        let items: Vec<LegacyAuthenticator> = serde_json::de::from_slice(&data)?;
        Ok(items)
    }

    fn restore_item(item: &Self::Item, model: &ProvidersModel) -> Result<()> {
        let issuer = item.tags.get(0).unwrap();
        info!(
            "Restoring account: {} - {} from LegacyAuthenticator",
            issuer, item.label
        );

        let provider = model.find_or_create(
            &issuer,
            item.period,
            item.method,
            None,
            item.algorithm,
            item.digits,
            otp::HOTP_DEFAULT_COUNTER,
        )?;
        let account = Account::create(&item.label, &item.secret, &provider)?;
        provider.add_account(&account);
        Ok(())
    }
}
