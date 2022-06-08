use super::{Restorable, RestorableItem};
use crate::models::{Algorithm, OTPMethod, OTPUri};
use anyhow::Result;
use gettextrs::gettext;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bitwarden {
    items: Vec<BitwardenItem>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitwardenItem {
    #[serde(rename = "name")]
    issuer: Option<String>,
    login: Option<BitwardenDetails>,
    #[serde(skip)]
    algorithm: Algorithm,
    #[serde(skip)]
    method: OTPMethod,
    #[serde(skip)]
    digits: Option<u32>,
    #[serde(skip)]
    period: Option<u32>,
    #[serde(skip)]
    counter: Option<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BitwardenDetails {
    username: Option<String>,
    totp: Option<String>,
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
    const ENCRYPTABLE: bool = false;
    const SCANNABLE: bool = false;

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

    fn restore_from_data(from: &[u8], _key: Option<&str>) -> Result<Vec<Self::Item>> {
        let bitwarden_root: Bitwarden = serde_json::de::from_slice(from)?;

        let mut items = Vec::new();

        for mut item in bitwarden_root.items {
            if let Some(ref login) = item.login {
                if let Some(ref totp) = login.totp {
                    if let Ok(uri) = OTPUri::from_str(totp) {
                        item.overwrite_with(uri);
                    }
                    items.push(item);
                }
            }
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bitwarden_restore_otpauth() {
        let bitwarden_data = r#"{
  "items": [
    {
      "id": "6095e880-06e0-4073-b38e-aade00f849d2",
      "organizationId": null,
      "folderId": null,
      "type": 1,
      "name": "test.com",
      "notes": null,
      "favorite": false,
      "login": {
        "uris": [
          {
            "match": null,
            "uri": "https://test.com"
          }
        ],
        "username": "test@testmail.com",
        "password": "äö^ 4234",
        "totp": "otpauth://totp/test.com:source.test_test%40testmail.com?secret=S22VG5VDNIUK2YIOMPNJ2ADNM3FNZSR2&issuer=test.com"
      },
      "collectionIds": null
    }
  ]
}"#;

        let bitwarden_items = Bitwarden::restore_from_slice(&bitwarden_data.as_bytes())
            .expect("Restoring from json should work");

        assert_eq!(bitwarden_items[0].account(), "test@testmail.com");
        assert_eq!(bitwarden_items[0].issuer(), "test.com");
        assert_eq!(
            bitwarden_items[0].secret(),
            "S22VG5VDNIUK2YIOMPNJ2ADNM3FNZSR2"
        );
        assert_eq!(bitwarden_items[0].period(), None);
        assert_eq!(bitwarden_items[0].algorithm(), Algorithm::default());
        assert_eq!(bitwarden_items[0].digits(), None);
        assert_eq!(bitwarden_items[0].counter(), None);
    }

    #[test]
    fn test_bitwarden_restore_totp_number() {
        let bitwarden_data = r#"{
  "items": [
    {
      "id": "6095e880-06e0-4073-b38e-aade00f849d2",
      "organizationId": null,
      "folderId": null,
      "type": 1,
      "name": "test.com",
      "notes": null,
      "favorite": false,
      "login": {
        "uris": [
          {
            "match": null,
            "uri": "https://test.com"
          }
        ],
        "username": "test@testmail.com",
        "password": "27ß4357ß2345%",
        "totp": "xkbu m5fw xxaa jqml 64qh yhi2 xdyf wjz2"
      },
      "collectionIds": null
    }
  ]
}"#;

        let bitwarden_items = Bitwarden::restore_from_data(&bitwarden_data.as_bytes())
            .expect("Restoring from json should work");

        assert_eq!(bitwarden_items[0].account(), "test@testmail.com");
        assert_eq!(bitwarden_items[0].issuer(), "test.com");
        assert_eq!(
            bitwarden_items[0].secret(),
            "xkbu m5fw xxaa jqml 64qh yhi2 xdyf wjz2"
        );
        assert_eq!(bitwarden_items[0].period(), None);
        assert_eq!(bitwarden_items[0].algorithm(), Algorithm::default());
        assert_eq!(bitwarden_items[0].digits(), None);
        assert_eq!(bitwarden_items[0].counter(), None);
    }
}
