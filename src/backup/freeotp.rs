use super::{Backupable, Restorable};
use crate::models::ProvidersModel;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreeOTP {}

impl Backupable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        "Authenticator".to_string()
    }

    fn subtitle() -> String {
        "Into a plain-text JSON file, compatible with FreeOTP+".to_string()
    }

    fn backup(model: ProvidersModel, into: gio::File) -> Result<()> {
        Ok(())
    }
}

impl Restorable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        "Authenticator".to_string()
    }

    fn subtitle() -> String {
        "From a plain-text JSON file, compatible with FreeOTP+".to_string()
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
