use super::{Backupable, Restorable};
use crate::models::{Account, Algorithm, HOTPAlgorithm, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gio::{FileExt, ListModelExt};
use glib::Cast;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AndOTP {
    pub secret: String,
    pub issuer: String,
    pub label: String,
    pub digits: i32,
    #[serde(rename = "type")]
    pub type_field: Algorithm,
    pub algorithm: HOTPAlgorithm,
    pub thumbnail: String,
    pub last_used: i64,
    pub used_frequency: i32,
    pub counter: Option<i32>,
    pub tags: Vec<String>,
    pub period: Option<i32>,
}

impl Backupable for AndOTP {
    fn identifier() -> String {
        "andotp".to_string()
    }

    fn title() -> String {
        gettext("andOTP")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text JSON file")
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

                let otp_item = AndOTP {
                    secret: account.token(),
                    issuer: provider.name(),
                    label: account.name(),
                    digits: provider.digits(),
                    type_field: provider.algorithm(),
                    algorithm: provider.hmac_algorithm(),
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
            gio::FileCreateFlags::REPLACE_DESTINATION,
            gio::NONE_CANCELLABLE,
        )?;

        Ok(())
    }
}

impl Restorable for AndOTP {
    fn identifier() -> String {
        "andotp".to_string()
    }

    fn title() -> String {
        gettext("andOTP")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        let (data, _) = from.load_contents(gio::NONE_CANCELLABLE)?;

        let items: Vec<AndOTP> = serde_json::de::from_slice(&data)?;
        items.iter().try_for_each(|item| -> anyhow::Result<()> {
            info!(
                "Restoring account: {} - {} from AndOTP",
                item.issuer, item.label
            );

            let provider = model.find_or_create(
                &item.issuer,
                item.period.unwrap_or_else(|| 30),
                item.type_field,
                None,
                item.algorithm,
                item.digits,
                item.counter.unwrap_or_else(|| 1),
            )?;

            let account = Account::create(&item.label, &item.secret, &provider)?;
            provider.add_account(&account);
            Ok(())
        });
        Ok(())
    }
}
