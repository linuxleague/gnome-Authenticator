use anyhow::Result;
use gettextrs::gettext;
use serde::Deserialize;

use super::{Restorable, RestorableItem};
use crate::models::{otp::encode_secret, Algorithm, Method};

#[derive(Deserialize)]
pub struct FreeOTPJSON {
    tokens: Vec<FreeOTPItem>,
}

#[derive(Deserialize)]
pub struct FreeOTPItem {
    algo: Algorithm,
    counter: Option<u32>,
    digits: Option<u32>,
    label: String,
    #[serde(rename = "issuerExt")]
    issuer: String,
    period: Option<u32>,
    secret: Vec<i16>,
    #[serde(rename = "type")]
    method: Method,
}

impl RestorableItem for FreeOTPItem {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.issuer.clone()
    }

    fn secret(&self) -> String {
        let secret = self
            .secret
            .iter()
            .map(|x| (x & 0xff) as u8)
            .collect::<Vec<_>>();
        encode_secret(&secret)
    }

    fn period(&self) -> Option<u32> {
        self.period
    }

    fn method(&self) -> Method {
        self.method
    }

    fn algorithm(&self) -> Algorithm {
        self.algo
    }

    fn digits(&self) -> Option<u32> {
        self.digits
    }

    fn counter(&self) -> Option<u32> {
        self.counter
    }
}

impl Restorable for FreeOTPJSON {
    const ENCRYPTABLE: bool = false;
    const SCANNABLE: bool = false;
    const IDENTIFIER: &'static str = "freeotp_json";
    type Item = FreeOTPItem;

    fn title() -> String {
        gettext("FreeOTP+")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file, compatible with FreeOTP+")
    }

    fn restore_from_data(from: &[u8], _key: Option<&str>) -> Result<Vec<Self::Item>> {
        let root: FreeOTPJSON = serde_json::de::from_slice(from)?;
        Ok(root.tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let data = std::fs::read_to_string("./src/backup/tests/freeotp_json.json").unwrap();
        let items = FreeOTPJSON::restore_from_data(data.as_bytes(), None).unwrap();

        assert_eq!(items[0].account(), "bar1");
        assert_eq!(items[0].issuer(), "foo1");
        assert_eq!(items[0].secret(), "AAAA2345");
        assert_eq!(items[0].period(), Some(30));
        assert_eq!(items[0].algorithm(), Algorithm::default());
        assert_eq!(items[0].method(), Method::default());
        assert_eq!(items[0].digits(), Some(6));
        assert_eq!(items[0].counter(), Some(0));

        assert_eq!(items[1].account(), "bar2");
        assert_eq!(items[1].issuer(), "foo2");
        assert_eq!(items[1].secret(), "BBBB2345");
        assert_eq!(items[1].period(), None);
        assert_eq!(items[1].algorithm(), Algorithm::default());
        assert_eq!(items[1].method(), Method::default());
        assert_eq!(items[1].digits(), None);
        assert_eq!(items[1].counter(), None);
    }
}

//
