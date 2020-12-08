use super::Restorable;
use crate::models::ProvidersModel;
use anyhow::Result;
use gettextrs::gettext;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyAuthenticator;

impl Restorable for LegacyAuthenticator {
    fn identifier() -> String {
        "authenticator_legacy".to_string()
    }

    fn title() -> String {
        gettext("Authenticator (Old)")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
