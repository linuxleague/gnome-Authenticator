use super::{Backupable, Restorable, RestorableItem};
use crate::models::{Account, Algorithm, OTPMethod, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gtk::{glib::Cast, prelude::*};
use serde::{Deserialize, Serialize};

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AndOTP {
    pub secret: String,
    pub issuer: String,
    pub label: String,
    pub digits: u32,
    #[serde(rename = "type")]
    pub method: OTPMethod,
    pub algorithm: Algorithm,
    pub thumbnail: String,
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
        self.counter
    }
}

impl Backupable for AndOTP {
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

    fn backup(model: &ProvidersModel, into: &gtk::gio::File) -> Result<()> {
        let mut items = Vec::new();

        for i in 0..model.get_n_items() {
            let provider = model.get_object(i).unwrap().downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.get_n_items() {
                let account = accounts
                    .get_object(j)
                    .unwrap()
                    .downcast::<Account>()
                    .unwrap();

                let otp_item = AndOTP {
                    secret: account.token(),
                    issuer: provider.name(),
                    label: account.name(),
                    digits: provider.digits(),
                    method: provider.method(),
                    algorithm: provider.algorithm(),
                    thumbnail: "".to_string(),
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

        into.replace_contents(
            content.as_bytes(),
            None,
            false,
            gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
            gtk::gio::NONE_CANCELLABLE,
        )?;

        Ok(())
    }
}

impl Restorable for AndOTP {
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

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::NONE_CANCELLABLE)?;

        let items: Vec<AndOTP> = serde_json::de::from_slice(&data)?;
        Ok(items)
    }
}
