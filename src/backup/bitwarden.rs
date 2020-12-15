use super::Restorable;
use crate::models::ProvidersModel;
use anyhow::Result;
use gettextrs::gettext;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bitwarden;

impl Restorable for Bitwarden {
    fn identifier() -> String {
        "bitwarden".to_string()
    }

    fn title() -> String {
        gettext("_Bitwarden")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
