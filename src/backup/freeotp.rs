use super::{Backupable, Restorable};
use crate::models::ProvidersModel;
use anyhow::Result;
use gettextrs::gettext;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreeOTP {}

impl Backupable for FreeOTP {
    fn identifier() -> String {
        "authenticator".to_string()
    }

    fn title() -> String {
        gettext("Authenticator")
    }

    fn subtitle() -> String {
        gettext("Into a plain-text JSON file, compatible with FreeOTP+")
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
        gettext("Authenticator")
    }

    fn subtitle() -> String {
        gettext("From a plain-text JSON file, compatible with FreeOTP+")
    }

    fn restore(model: ProvidersModel, from: gio::File) -> Result<()> {
        Ok(())
    }
}
