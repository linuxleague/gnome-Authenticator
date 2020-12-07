use super::Restorable;
use crate::models::ProvidersModel;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bitwarden;

impl Restorable for Bitwarden {
    fn identifier() -> String {
        "bitwarden".to_string()
    }

    fn title() -> String {
        "Bitwarden".to_string()
    }

    fn subtitle() -> String {
        "From a plain-text JSON file".to_string()
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
