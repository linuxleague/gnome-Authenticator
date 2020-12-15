use super::{Backupable, Restorable};
use crate::models::{Account, OTPUri, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gio::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreeOTP {}

impl Backupable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("_Authenticator")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text file, compatible with FreeOTP+")
    }

    fn backup(model: ProvidersModel, into: gio::File) -> Result<()> {
        let mut items: Vec<String> = Vec::new();

        for i in 0..model.get_n_items() {
            let provider = model.get_object(i).unwrap().downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.get_n_items() {
                let account = accounts
                    .get_object(j)
                    .unwrap()
                    .downcast::<Account>()
                    .unwrap();

                items.push(account.otp_uri().into());
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
        gettext("A_uthenticator")
    }

    fn subtitle() -> String {
        gettext("From a plain-text file, compatible with FreeOTP+")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        let (data, _) = from.load_contents(gio::NONE_CANCELLABLE)?;
        let uris = String::from_utf8(data)?;

        uris.split('\n')
            .into_iter()
            .try_for_each(|uri| -> Result<()> {
                println!("{:#?}", uri);
                let otp_uri = OTPUri::from_str(uri)?;
                let provider = model.find_or_create(
                    &otp_uri.issuer,
                    otp_uri.period.unwrap_or(30),
                    otp_uri.method,
                    None,
                    otp_uri.algorithm,
                    otp_uri.digits.unwrap_or(6),
                    otp_uri.counter.unwrap_or(1),
                )?;

                let account = Account::create(&otp_uri.label, &otp_uri.secret, &provider)?;
                provider.add_account(&account);

                Ok(())
            });
        Ok(())
    }
}
