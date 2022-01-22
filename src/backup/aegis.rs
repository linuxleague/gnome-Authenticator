use super::{Backupable, Restorable, RestorableItem};
use crate::models::{Account, Algorithm, OTPMethod, Provider, ProvidersModel};
use anyhow::Result;
use anyhow::Context;
use gettextrs::gettext;
use gtk::{glib::Cast, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aegis {
    pub version: u32,
    #[serde(flatten)]
    pub header: std::collections::HashMap<String, serde_json::Value>,
    pub db: AegisDatabase,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AegisDatabase {
    pub version: u32,
    pub entries: Vec<AegisItem>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AegisItem {
    #[serde(rename = "type")]
    pub method: OTPMethod,
    // UUID is omitted
    #[serde(rename = "name")]
    pub label: String,
    pub issuer: String,
    // Groups in aegis are imported as tags. Is this what we want?
    #[serde(rename = "group")]
    pub tags: Option<String>,
    // Note is omitted
    #[serde(rename = "icon")]
    // TODO: Aegis encodes icons as JPEG's encoded in Base64 with padding. Does authenticator support
    // this?
    pub thumbnail: Option<String>,
    pub info: AegisDetail,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AegisDetail {
    pub secret: String,
    #[serde(rename = "algo")]
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: Option<u32>,
    pub counter: Option<u32>,
}

impl Aegis {
    fn restore_from_slice(data: &[u8]) -> Result<Vec<AegisItem>> {
        // TODO check whether file / database is encrypted by aegis
        let aegis_root: Aegis = serde_json::de::from_slice(&data)?;

        // Check for correct aegis file version and correct database version.
        if aegis_root.version != 1 {
            anyhow::bail!(
                "Aegis file version expected to be 1. Found {} instead.",
                aegis_root.version
                );
        }
        if aegis_root.db.version != 2 {
            anyhow::bail!(
                "Aegis file version expected to be 2. Found {} instead.",
                aegis_root.db.version
                );
        }

        Ok(aegis_root.db.entries)
    }
}


impl RestorableItem for AegisItem {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn secret(&self) -> String {
        self.info.secret.clone()
    }

    fn period(&self) -> Option<u32> {
        self.info.period
    }

    fn method(&self) -> OTPMethod {
        self.method
    }

    fn algorithm(&self) -> Algorithm {
        self.info.algorithm
    }

    fn digits(&self) -> Option<u32> {
        Some(self.info.digits)
    }

    fn counter(&self) -> Option<u32> {
        self.info.counter
    }
}

//impl Backupable for Aegis {
//    fn identifier() -> String {
//        "aegis".to_string()
//    }
//
//    fn title() -> String {
//        // Translators: This is for making a backup for the andOTP Android app.
//        gettext("a_ndOTP")
//    }
//
//    fn subtitle() -> String {
//        gettext("Into a plain-text JSON file")
//    }
//
//    fn backup(model: &ProvidersModel, into: &gtk::gio::File) -> Result<()> {
//        let mut items = Vec::new();
//
//        for i in 0..model.n_items() {
//            let provider = model.item(i).unwrap().downcast::<Provider>().unwrap();
//            let accounts = provider.accounts_model();
//
//            for j in 0..accounts.n_items() {
//                let account = accounts.item(j).unwrap().downcast::<Account>().unwrap();
//
//                let otp_item = AndOTP {
//                    secret: account.token(),
//                    issuer: provider.name(),
//                    label: account.name(),
//                    digits: provider.digits(),
//                    method: provider.method(),
//                    algorithm: provider.algorithm(),
//                    thumbnail: "".to_string(),
//                    last_used: 0,
//                    used_frequency: 0,
//                    counter: Some(account.counter()),
//                    tags: vec![],
//                    period: Some(provider.period()),
//                };
//                items.push(otp_item);
//            }
//        }
//
//        let content = serde_json::ser::to_string_pretty(&items)?;
//
//        into.replace_contents(
//            content.as_bytes(),
//            None,
//            false,
//            gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
//            gtk::gio::Cancellable::NONE,
//        )?;
//
//        Ok(())
//    }
//}

impl Restorable for Aegis {
    type Item = AegisItem;

    fn identifier() -> String {
        "aegis".to_string()
    }

    fn title() -> String {
        // Translators: This is for restoring a backup from the aegis Android app.
        gettext("aegis")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gtk::gio::Cancellable::NONE)?;
        Aegis::restore_from_slice(&data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_unencrypted_file() {
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": null,
        "params": null
    },
    "db": {
        "version": 2,
        "entries": [
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "Bob",
                "issuer": "Google",
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            },
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "Alice",
                "issuer": "Element One",
                "group": "social",
                "note": "",
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            }
        ]
    }
}"#;

        let aegis_items = Aegis::restore_from_slice(&aegis_data.as_bytes())
            .expect("Restoring from json should work");

        assert_eq!(aegis_items[0].account(), "Bob");
        assert_eq!(aegis_items[0].issuer(), "Google");
        assert_eq!(
            aegis_items[0].secret(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
        );
        assert_eq!(aegis_items[0].period(), Some(30));
        assert_eq!(aegis_items[0].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[0].digits(), Some(6));
        assert_eq!(aegis_items[0].counter(), None);
        assert_eq!(aegis_items[0].method(), OTPMethod::TOTP);

        assert_eq!(aegis_items[1].account(), "Alice");
        assert_eq!(aegis_items[1].issuer(), "Element One");
        assert_eq!(
            aegis_items[1].secret(),
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
        );
        assert_eq!(aegis_items[1].period(), Some(30));
        assert_eq!(aegis_items[1].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[1].digits(), Some(6));
        assert_eq!(aegis_items[1].counter(), None);
        assert_eq!(aegis_items[1].method(), OTPMethod::TOTP);
    }
}
