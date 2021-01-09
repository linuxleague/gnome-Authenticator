use std::fmt::Debug;

use crate::models::ProvidersModel;
use anyhow::Result;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Operation {
    Backup,
    Restore,
}

pub trait Restorable: Sized {
    type Item: Debug;

    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `restore.$identifier` action
    fn identifier() -> String;
    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>>;
    fn restore_item(item: &Self::Item, model: &ProvidersModel) -> Result<()>;
}

pub trait Backupable: Sized {
    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `backup.$identifier` action
    fn identifier() -> String;
    fn backup(model: &ProvidersModel, into: &gtk::gio::File) -> Result<()>;
}

mod andotp;
mod bitwarden;
mod freeotp;
mod legacy;
pub use self::{
    andotp::AndOTP, bitwarden::Bitwarden, freeotp::FreeOTP, legacy::LegacyAuthenticator,
};
