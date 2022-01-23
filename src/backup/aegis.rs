//! Aegis Import/Export Module
//!
//! See https://github.com/beemdevelopment/Aegis/blob/master/docs/vault.md for a description of the
//! aegis vault format.

use super::{Backupable, Restorable, RestorableItem};
use crate::models::{Account, Algorithm, OTPMethod, Provider, ProvidersModel};
use anyhow::Result;
use gettextrs::gettext;
use gtk::{glib::Cast, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aegis {
    pub version: u32,
    pub header: std::collections::HashMap<String, serde_json::Value>,
    pub db: AegisDatabase,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AegisDatabase {
    Encrypted(String),
    Plaintext {
        version: u32,
        entries: Vec<AegisItem>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AegisItem {
    #[serde(rename = "type")]
    pub method: OTPMethod,
    // UUID is omitted
    #[serde(rename = "name")]
    pub label: String,
    pub issuer: String,
    // Groups in aegis are imported as tags. Is this what we want?
    // TODO tags are not imported/exported right now.
    #[serde(rename = "group")]
    pub tags: Option<String>,
    // Note is omitted
    // Icon:
    // TODO: Aegis encodes icons as JPEG's encoded in Base64 with padding. Does authenticator support
    // this?
    // TODO tags are not importet/exported right now.
    #[serde(rename = "icon")]
    pub thumbnail: Option<String>,
    pub info: AegisDetail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        // Additionally, we check whether the file is encrypted. We can't open / decrypt them yet,
        // because there is no UI possibility to enter the password.
        if aegis_root.version != 1 {
            anyhow::bail!(
                "Aegis file version expected to be 1. Found {} instead.",
                aegis_root.version
            );
        }

        match aegis_root.db {
            AegisDatabase::Encrypted(_) => anyhow::bail!(
                "Aegis file is encrypted. Authenticator supports only plaintext files."
            ),
            AegisDatabase::Plaintext { version, entries } if version == 2 => return Ok(entries),
            AegisDatabase::Plaintext { version, .. } => anyhow::bail!(
                "Aegis file version expected to be 2. Found {} instead.",
                version
            ),
        }
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

impl Backupable for Aegis {
    fn identifier() -> String {
        "aegis".to_string()
    }

    fn title() -> String {
        // Translators: This is for making a backup for the aegis Android app.
        gettext("aegis")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text JSON file")
    }

    fn backup(model: &ProvidersModel, into: &gtk::gio::File) -> Result<()> {
        let mut items = Vec::new();

        for i in 0..model.n_items() {
            let provider = model.item(i).unwrap().downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.n_items() {
                let account = accounts.item(j).unwrap().downcast::<Account>().unwrap();

                let aegis_detail = AegisDetail {
                    secret: account.token(),
                    algorithm: provider.algorithm(),
                    digits: provider.digits(),
                    period: Some(provider.period()),
                    counter: Some(account.counter()),
                };

                let aegis_item = AegisItem {
                    method: provider.method(),
                    label: account.name(),
                    issuer: provider.name(),
                    tags: None,
                    thumbnail: None,
                    info: aegis_detail,
                };

                items.push(aegis_item);
            }
        }

        // Create structure around items
        let aegis_db = AegisDatabase::Plaintext {
            version: 2,
            entries: items,
        };
        let aegis_root = Aegis {
            version: 1,
            header: std::collections::HashMap::from([
                (String::from("slots"), serde_json::Value::Null),
                (String::from("params"), serde_json::Value::Null),
            ]),
            db: aegis_db,
        };

        let content = serde_json::ser::to_string_pretty(&aegis_root)?;

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
        assert_eq!(aegis_items[0].secret(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567");
        assert_eq!(aegis_items[0].period(), Some(30));
        assert_eq!(aegis_items[0].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[0].digits(), Some(6));
        assert_eq!(aegis_items[0].counter(), None);
        assert_eq!(aegis_items[0].method(), OTPMethod::TOTP);

        assert_eq!(aegis_items[1].account(), "Alice");
        assert_eq!(aegis_items[1].issuer(), "Element One");
        assert_eq!(aegis_items[1].secret(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567");
        assert_eq!(aegis_items[1].period(), Some(30));
        assert_eq!(aegis_items[1].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[1].digits(), Some(6));
        assert_eq!(aegis_items[1].counter(), None);
        assert_eq!(aegis_items[1].method(), OTPMethod::TOTP);
    }

    #[test]
    #[should_panic]
    fn detect_encrypted_file() {
        // See https://github.com/beemdevelopment/Aegis/blob/master/app/src/test/resources/com/beemdevelopment/aegis/importers/aegis_encrypted.json
        // for this example file.
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": [
            {
                "type": 1,
                "uuid": "a8325752-c1be-458a-9b3e-5e0a8154d9ec",
                "key": "491d44550430ba248986b904b8cffd3a6c5755d176ac877bd11b82c934225017",
                "key_params": {
                    "nonce": "e9705513ba4951fa7a0608d2",
                    "tag": "931237af257b83c693ddb8f9a7eddaf0"
                },
                "n": 32768,
                "r": 8,
                "p": 1,
                "salt": "27ea9ae53fa2f08a8dcd201615a8229422647b3058f9f36b08f9457e62888be1",
                "repaired": true
            }
        ],
        "params": {
            "nonce": "095fd13dee336fa56b4634ff",
            "tag": "5db2470edf2d12f82a89ae7f48ccd50c"
        }
    },
    "db": "RtGfUrZ01nzRnvHjPJGyWjfa6shQ7NYwa491CgAWNBM8OeGZVIHhnDAVlVWNlSoq2V097p5Yq5m+SFl5g9nBBBQBNePQnj6CCvu1NfNtoA6R3hyp77gd+e+O2MRnOGH1Z1laV2Tl6p3q8IUHWgAJ36LbUxiCXmfh7bWm198uA4bgLwrEmo04MrqeYXggLuXrJrp6dUJQFD72dgoPbHijlSycY5GLel3ZbAXRsUHszd+xdywpj7\/TYa4OYFel0M0QcCpsKA1LRQz365X9OXPJdTsmVyR4dJ6x5RIVeh39lAYKUf7T4w7BLC8taST5m4J\/VXDueKbvg8R13bNWF0aRHUgeuI9BNzMZINJlzKFKNRknTaJ\/1kEUU0sLkgcaVkX\/DVTGG+pWi5MHijicrK0i4LHN3CUwV2\/\/ZNJCGXM5ErsKMOnJfma52gMdifPiXU317Klvc5oOZFYGnhbhJ2WtPIuqjdvnfuLat2JxA7Xx3LqquRWGL2113yjzVzGBDCVY6iIdedBEgH8CGD826\/3R3m6dR5sfSggQ2SbtQA\/DZNhLSNSU+bfNScVQvUWfR2Lf7Q\/4FR\/xATAQJ9IIBeL+w2ErLUPjURocFXup5YOBHxFdDjZ2FqhbAq4h3Zn\/BJ57xUcYEA+YtP5uOP2lQwUh\/0vFWizDVotzraO8tZiBZBsODyb69eJrXNwFbIjeUczY6wrJs1+676IilbCsmtoYvWEpUZF4hIi7TYAD+nyXX\/olrkog9omWZk8R7hJ9KRDfckXEc\/XSzWhk3Kmfa7pRNh9wYZsaR7VPZGZebQMuUKfRRci2qMsZOJvQsDBJvVze0xW9SqiySDgGyRX\/DwzuaZEGZZriaLf6ox7LwY2Qi6QpYOYbAaEaXAesCR1DPxFfGKsUHVjF8hKA6ZBXDXdqM3Y+14naIOH9S7UzYn32botoVLOykSjnW6z6M0ZPkz3dwowMJiVQcyD7p+9p4J6f1S81pFS7DP+jF+PTyC3c3q\/dwFhNdoG6iV9eQEAxjUi6MpzvFRsk9RsLcQqYgzJGmRjYeXlKH8k8tTu1A4puo6w3Daz8hZz9NafMgMsuqY0oKVLgdNqFz8yVMsxYfBW\/oW56SuQyyVWyxXjXmbk1vpYCTL5kXvIZWoTmBRRDb0ay5S\/dlD6z\/WR45\/C4AwcCE9m4Yf3zisRNa7AqWLVgkmJxFdfJxjiuPtUIK79s+lIJkyRENEqkvm809qIxDhkQzY8zcCt4oXCEbJUfSG4awBs1VvilJIwe6qi0bNtqXtAb5TctgxTh29A9oGlsRG4o8sHqA1mtjp5QiLWp5Hh6rOH95W6+fnBiOW+Iw0evBTduroWvx37HBTktJz79zGe0l3c0Y6VmiFvB7knmT2CrgP7woRkxGbXxdE9zMPQJM9ursD538MVDdD\/0tdkxHxilt47f1DPo2CKUWU8Q1KMm1zLXfVO8BbGUWIv4YeDKHfMUL\/HcStv5VJY+LbnOEjzGT4e1\/avSQmqBL4G9XNkYmyMhC8tlLQcmMMH4bNfPOO3vi5Pb5E7XveSgxlOHs4F0+nqxnFOAu8494MEtx6u5+B7d8LI\/DhEO5zTDwE+THiKej6vCsFxTZ519rm67HycOwRR4LKrwfDeUEK3X1PzryOD5zcv3PMcSBgZ8EWvTfZ9ygKP8BmRQRpydTbSt8Hj5fTUuajADCP0Ggw+6G7n+5FhExJNd+o9D8d4KgLPOe08M8InW7pLB389TWtSo4v3VNjcmmJNQ26wlPkhO\/xBU1URFR0fXU3eCO+w++IMt\/fOSqSpNF9bWElfWHIQ23ntxVke\/hR9j\/GG3tHGxYS5pL42sJF\/Re\/UlUJTGSQP6up2xVYs6gncQ0zACDOPjLQmQzYhz\/hr8S6EjYfK++yLZmRTjEI7xT9u\/B5YLyOQCYVTaF\/pDEegjsehXM3qJBfsA+XY7F9TRsmM\/MSVaPDkdIJ7zvL9xtaF6bXdZoZ6po3ml8uu41pSkNmMKgyEy5E0UQUTWMPLC8drUoQ\/KWQnVIN6HUXGBjYy6aax\/LYZaBcbZi97FHK0h+wsx3WN\/uQozNkQjwGYE8fwYxRYh1RaFi5PkiCM505ib7e82Yuts0l+cBb6nG1IruDplg9BD\/G9w4vVDePEikhcPyY\/p7AZ4i7u\/bL2YKlbE3HyJa+7dkbWJgGidtRZgu+Fdl2T\/rrRJ4+lVaKPVKGKT7ItZdIeitIYUdRxCzrOf1ItZCC8BWa4PElDAjj2yDNmMYRpXJBe3gQHWs\/H5SZgFuwsfCu23uzNRQYib8SuwIJQDvPiXo7m4oIySO8VyvemcExlbXSlbZbvwVxYavTVfcUpAXI6qlsg2jjk+JZahfKrWNC5COZPdVjdAXCoiKU+HBPmEFCwQv\/7zlSBEiI2piyqd+MPwnP63RdGO+oXYid6hn4Nm8kcOhtRyvYm95p66jzGlEugsfxJCED7MTh3XShqa2tt4lFG25icllzTvIJboRkz5oIB4dZVS9+q2TgGUoX7UCpobD8WkHo\/y0cpTuZr8vzXqx2fObxzPNoVgxJmp9E06G2bhMVHPpT17xbfq\/KhJJn7k1S0sfXPG+SmYlX4U7zNSe1M7JXtLf3uVOLz7Ccjp3yvcdq8nRmVym3Zwsz+vv57FA2A0dy3Db97ypJa9HGaxnnYIZHHzep0gJCeeIKE9L32zGCoUg+cPu9B2lPEgIr64iGiuvKSRwNQpOBktM6qqjQntE0Me6mh426irFQ\/3tcfH9a4lZEwwuU1X+lUBUWQp3n5Ej4BSJEs8E6H0EjBvyk69q3qjy5yi7ROVRis6y6S1v4er77RHQUf3phK5354VJHrp9pR926t5qngH5RVF4eljwtXDs3MkejADJ6stBHa\/w7FcbUClO8U+S4Bidxb3mZCiZkUVTpbzvBfYAiQvAfdkMa49o3a5DXKsbXyUPrmr6fWRfM1fS0Ehp0lUv6BDj0yR13CLMpKDU4GfDrl8UEvwh7gwtBRkuaBFzyMtd3NeE7kIGf9vFs6MEl2dmMDFSDid7MdVSDVTlhaAtp+zsRejKW3OQr5n051FzkUsIFGty9AWOkwjZCbstHYCOtyJnsnXP1i9lRDFBgPpFgmDD+bzzg0g9AOAxzqTiLF7bb1jejfe5qVr5V9+7zLpwRLiYaLkNOmpsqvNMuYVwdqTp6nyoougdgBlvve3EG0k09sFKi2Ep9lq+QkS7zGre2jJDrqgdC08+V4PXHYkP3V3Zjgn1x6RfQ2PE+2zvk1GGEgzcNww3byoYw0Ra5qS5yftMy\/2WahbA8fjUYvtmksFH8VjN3yasZt3sdQLWtv8qXxZscy+pCyjTdyxW+ddFnrWuqMIV3jbGMvngq6dL\/n5+DumjbA1gmBJVOpmyEsc1iwHDS36cNnyi1htGFO\/6\/Va4YPYK7dG6LY387UoBUU9Q9ijrBrSGpzPWYmXBLZ8e1MMPfHIN1WsaTgYO9leg3MAJTjQFTFrQ5dguYpWhlm2sWJT45jrda4uWqduB+aQLzYRWhEDBFzPV3ZgIe0SB+7h04Vm0Pu\/LDRvqaolpZ86CEm+zgjBOKeEGFwzTXxH\/5pBoca1bZ6wvsbVZxJNBeH8\/w=="
}"#;

        let aegis_items = Aegis::restore_from_slice(&aegis_data.as_bytes())
            .expect("Reading encrypted file should fail.");
    }

    // TODO: add tests for importing
}
