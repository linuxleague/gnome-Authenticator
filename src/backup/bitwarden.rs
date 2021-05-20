use std::str::FromStr;

use super::{Restorable, RestorableItem};
use crate::models::{Algorithm, OTPMethod, OTPUri};
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
    #[serde(rename = "name")]
    pub issuer: Option<String>,
    pub login: Option<BitwardenDetails>,
    #[serde(skip_serializing)]
    pub algorithm: Algorithm,
    #[serde(skip_serializing)]
    pub method: OTPMethod,
    #[serde(skip_serializing)]
    pub digits: Option<u32>,
    #[serde(skip_serializing)]
    pub period: Option<u32>,
    #[serde(skip_serializing)]
    pub counter: Option<u32>,
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
        if let Some(issuer) = self.issuer.clone() {
            issuer
        } else {
            gettext("Unknown issuer")
        }
    }

    fn secret(&self) -> String {
        self.login.clone().unwrap().totp.unwrap()
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

impl BitwardenItem {
    fn overwrite_with(&mut self, uri: OTPUri) {
        if self.issuer.is_none() {
            self.issuer = Some(uri.issuer());
        }

        if let Some(ref mut login) = self.login {
            login.totp = Some(uri.secret());
        } else {
            self.login = Some(BitwardenDetails {
                username: None,
                totp: Some(uri.secret()),
            });
        }

        self.algorithm = uri.algorithm();
        self.method = uri.method();
        self.digits = uri.digits();
        self.period = uri.period();
        self.counter = uri.counter();
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

        let mut items = Vec::new();

        for mut item in bitwarden_root.items {
            if let Some(ref login) = item.login {
                if let Some(ref totp) = login.totp {
                    if let Ok(uri) = OTPUri::from_str(&totp) {
                        item.overwrite_with(uri);
                    }
                    items.push(item);
                }
            }
        }

        Ok(items)
    }
}
