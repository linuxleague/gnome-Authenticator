use super::{Backupable, Restorable};
use crate::models::{Account, OTPUri, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeOTP {}

impl Backupable for FreeOTP {
    const ENCRYPTABLE: bool = false;

    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("_Authenticator")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text file, compatible with FreeOTP+")
    }

    fn backup(model: &ProvidersModel, into: &gtk::gio::File, _key: Option<&str>) -> Result<()> {
        let mut items: Vec<String> = Vec::new();

        for i in 0..model.n_items() {
            let provider = model.item(i).unwrap().downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.n_items() {
                let account = accounts.item(j).unwrap().downcast::<Account>().unwrap();

                items.push(account.otp_uri().into());
            }
        }

        let content = items.join("\n");

        into.replace_contents(
            content.as_bytes(),
            None,
            false,
            gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
            gtk::gio::Cancellable::NONE,
        )?;

        Ok(())
    }
}

impl Restorable for FreeOTP {
    const ENCRYPTABLE: bool = false;
    const SCANNABLE: bool = false;

    type Item = OTPUri;

    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("A_uthenticator")
    }

    fn subtitle() -> String {
        gettext("From a plain-text file, compatible with FreeOTP+")
    }

    fn restore_from_data(from: &[u8], _key: Option<&str>) -> Result<Vec<Self::Item>> {
        let uris = String::from_utf8(from.into())?;

        let items = uris
            .split('\n')
            .into_iter()
            .filter_map(|uri| OTPUri::from_str(uri).ok())
            .collect::<Vec<OTPUri>>();

        Ok(items)
    }
}
