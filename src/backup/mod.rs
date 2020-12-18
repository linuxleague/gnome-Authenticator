use crate::models::ProvidersModel;
use anyhow::Result;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Operation {
    Backup,
    Restore,
}

pub trait Restorable {
    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `restore.$identifier` action
    fn identifier() -> String;
    fn restore(model: ProvidersModel, from: gtk::gio::File) -> Result<()>;
}

pub trait Backupable {
    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `backup.$identifier` action
    fn identifier() -> String;
    fn backup(model: ProvidersModel, into: gtk::gio::File) -> Result<()>;
}

mod andotp;
mod bitwarden;
mod freeotp;
mod legacy;
pub use self::{
    andotp::AndOTP, bitwarden::Bitwarden, freeotp::FreeOTP, legacy::LegacyAuthenticator,
};
