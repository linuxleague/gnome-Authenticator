use super::{Backupable, Restorable};
use crate::models::{Account, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gio::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreeOTP {}

impl Backupable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("Authenticator")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text JSON file, compatible with FreeOTP+")
    }

    fn backup(model: ProvidersModel, into: gio::File) -> Result<()> {
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

                items.push(account.otp_uri());
            }
        }

        let content = items.join("\n");

        into.replace_contents(
            content.as_bytes(),
            None,
            false,
            gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::NONE_CANCELLABLE,
        )?;

        Ok(())
    }
}

impl Restorable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("Authenticator")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file, compatible with FreeOTP+")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
