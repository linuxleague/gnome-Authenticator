use anyhow::Result;
use gettextrs::gettext;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Backupable, Restorable, RestorableItem};
use crate::models::{Account, Algorithm, Method, Provider, ProvidersModel};

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize)]
pub struct AndOTP {
    pub secret: String,
    pub issuer: String,
    pub label: String,
    pub digits: u32,
    #[serde(rename = "type")]
    pub method: Method,
    pub algorithm: Algorithm,
    pub thumbnail: Option<String>,
    pub last_used: i64,
    pub used_frequency: i32,
    pub counter: Option<u32>,
    pub tags: Vec<String>,
    pub period: Option<u32>,
}

impl RestorableItem for AndOTP {
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
        self.counter
    }
}

impl Backupable for AndOTP {
    const ENCRYPTABLE: bool = false;

    fn identifier() -> String {
        "andotp".to_string()
    }

    fn title() -> String {
        // Translators: This is for making a backup for the andOTP Android app.
        gettext("a_ndOTP")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text JSON file")
    }

    fn backup(model: &ProvidersModel, _key: Option<&str>) -> Result<Vec<u8>> {
        let mut items = Vec::new();

        for i in 0..model.n_items() {
            let provider = model.item(i).and_downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.n_items() {
                let account = accounts.item(j).and_downcast::<Account>().unwrap();

                let otp_item = AndOTP {
                    secret: account.token(),
                    issuer: provider.name(),
                    label: account.name(),
                    digits: provider.digits(),
                    method: provider.method(),
                    algorithm: provider.algorithm(),
                    thumbnail: None,
                    last_used: 0,
                    used_frequency: 0,
                    counter: Some(account.counter()),
                    tags: vec![],
                    period: Some(provider.period()),
                };
                items.push(otp_item);
            }
        }

        let content = serde_json::ser::to_string_pretty(&items)?;
        Ok(content.as_bytes().to_vec())
    }
}

impl Restorable for AndOTP {
    const ENCRYPTABLE: bool = false;
    const SCANNABLE: bool = false;

    type Item = Self;

    fn identifier() -> String {
        "andotp".to_string()
    }

    fn title() -> String {
        // Translators: This is for restoring a backup from the andOTP Android app.
        gettext("an_dOTP")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore_from_data(from: &[u8], _key: Option<&str>) -> Result<Vec<Self::Item>> {
        let items: Vec<AndOTP> = serde_json::de::from_slice(from)?;
        Ok(items)
    }
}
