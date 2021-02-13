use super::{Restorable, RestorableItem};
use crate::models::{Algorithm, OTPMethod};
use anyhow::Result;
use gettextrs::gettext;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bitwarden {
    pub encrypted: bool,
    pub items: Vec<BitwardenItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitwardenItem {
    pub name: Option<String>,
    pub login: Option<BitwardenDetails>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitwardenDetails {
    pub username: Option<String>,
    pub totp: Option<String>,
}

impl RestorableItem for BitwardenItem {
    fn account(&self) -> String {
        if let Some(account) = self
            .login
            .as_ref()
            .and_then(|login| login.username.as_ref())
        {
            account.clone()
        } else {
            gettext("Unknown account")
        }
    }

    fn issuer(&self) -> String {
        if let Some(issuer) = self.name.clone() {
            issuer
        } else {
            gettext("Unknown issuer")
        }
    }

    fn secret(&self) -> String {
        self.login.clone().unwrap().totp.clone().unwrap()
    }

    fn period(&self) -> Option<u32> {
        None
    }

    fn method(&self) -> OTPMethod {
        OTPMethod::TOTP
    }

    fn algorithm(&self) -> Algorithm {
        Algorithm::SHA1
    }

    fn digits(&self) -> Option<u32> {
        None
    }

    fn counter(&self) -> Option<u32> {
        None
    }
}

impl Restorable for Bitwarden {
    type Item = BitwardenItem;

    fn identifier() -> String {
        "bitwarden".to_string()
    }

    fn title() -> String {
        // Translators: This is for restoring a backup from Bitwarden.
        gettext("_Bitwarden")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::NONE_CANCELLABLE)?;

        let bitwarden_root: Bitwarden = serde_json::de::from_slice(&data)?;
        let items = bitwarden_root
            .items
            .into_iter()
            // Only take the fields where at least the totp secret is present
            .filter(|item| {
                item.login
                    .as_ref()
                    .and_then(|login| login.totp.as_ref())
                    .is_some()
            })
            .collect();
        Ok(items)
    }
}
