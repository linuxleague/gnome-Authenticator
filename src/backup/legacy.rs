use super::Restorable;
use crate::models::ProvidersModel;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyAuthenticator;

impl Restorable for LegacyAuthenticator {
    fn identifier() -> String {
        "authenticator_legacy".to_string()
    }

    fn title() -> String {
        "Authenticator (Old)".to_string()
    }

    fn subtitle() -> String {
        "From a plain-text JSON file".to_string()
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
