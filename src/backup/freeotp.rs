use super::{Backupable, Restorable};
use crate::models::{otp, Account, OTPUri, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gtk::prelude::*;
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

    fn backup(model: &ProvidersModel, into: &gtk::gio::File) -> Result<()> {
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
            gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
            gtk::gio::NONE_CANCELLABLE,
        )?;

        Ok(())
    }
}

impl Restorable for FreeOTP {
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

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::NONE_CANCELLABLE)?;
        let uris = String::from_utf8(data)?;

        let items = uris
            .split('\n')
            .into_iter()
            .map(|uri| OTPUri::from_str(uri))
            .filter(|uri| uri.is_ok())
            .map(|uri| uri.unwrap())
            .collect::<Vec<OTPUri>>();
        Ok(items)
    }

    fn restore_item(item: &Self::Item, model: &ProvidersModel) -> Result<()> {
        let provider = model.find_or_create(
            &item.issuer,
            item.period.unwrap_or(otp::TOTP_DEFAULT_PERIOD),
            item.method,
            None,
            item.algorithm,
            item.digits.unwrap_or(otp::DEFAULT_DIGITS),
            item.counter.unwrap_or(otp::HOTP_DEFAULT_COUNTER),
        )?;

        let account = Account::create(&item.label, &item.secret, &provider)?;
        provider.add_account(&account);
        Ok(())
    }
}
