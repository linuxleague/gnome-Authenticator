use super::Restorable;
use crate::models::ProvidersModel;
use anyhow::Result;
use gettextrs::gettext;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bitwarden;
/*
impl Restorable for Bitwarden {
    type Item = Self;

    fn identifier() -> String {
        "bitwarden".to_string()
    }

    fn title() -> String {
        gettext("_Bitwarden")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file")
    }

    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>> {
        Ok(Vec::new())
    }

    fn restore_item(item: &Self::Item, model: &ProvidersModel) -> Result<()> {
        Ok(())
    }
} */
