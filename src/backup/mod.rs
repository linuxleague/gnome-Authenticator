use std::fmt::Debug;

use crate::models::{Account, Algorithm, OTPMethod, ProvidersModel};
use anyhow::Result;
use gtk::{gio, gio::prelude::*};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Operation {
    Backup,
    Restore,
}

pub trait Restorable: Sized {
    /// Indicates that the GUI might need to prompt for a password.
    const ENCRYPTABLE: bool = false;

    /// Indicates that the GUI needs to show a QR code scanner.
    const SCANNABLE: bool = false;

    type Item: RestorableItem;

    fn title() -> String;
    fn subtitle() -> String;
    fn identifier() -> String;

    /// Restore many items from a slice of data, optionally using a key to unencrypt it.
    ///
    /// If `key` is `None`, then the implementation should assume that the slice is unencrypted, and
    /// error if it only supports encrypted slices.
    fn restore_from_data(from: &[u8], key: Option<&str>) -> Result<Vec<Self::Item>>;

    /// Restore many items from a file, optiontally using a key to unencrypt it.
    ///
    /// If `key` is `None`, then the implementation should assume that the file is unencrypted, and
    /// error if it only supports encrypted files.
    ///
    /// By default, this method reads the file and passes the files content to
    /// `Self::restore_from_data`.
    fn restore_from_file(from: &gio::File, key: Option<&str>) -> Result<Vec<Self::Item>> {
        let (data, _) = from.load_contents(gio::Cancellable::NONE)?;
        Self::restore_from_data(&*data, key)
    }
}

pub trait RestorableItem: Debug {
    fn account(&self) -> String;
    fn issuer(&self) -> String;
    fn secret(&self) -> String;
    fn period(&self) -> Option<u32>;
    fn method(&self) -> OTPMethod;
    fn algorithm(&self) -> Algorithm;
    fn digits(&self) -> Option<u32>;
    fn counter(&self) -> Option<u32>;

    fn restore(&self, provider: &ProvidersModel) -> Result<()> {
        let provider = provider.find_or_create(
            &self.issuer(),
            self.period(),
            self.method(),
            None,
            self.algorithm(),
            self.digits(),
            self.counter(),
            None,
            None,
        )?;

        let account = Account::create(
            &self.account(),
            &self.secret(),
            self.counter(),
            &provider,
        )?;

        provider.add_account(&account);

        Ok(())
    }
}

pub trait Backupable: Sized {
    /// Indicates that the GUI might need to prompt for a password.
    const ENCRYPTABLE: bool = false;

    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `backup.$identifier` action
    fn identifier() -> String;
    // if no key is provided the backup code should save it as plain text
    fn backup(provider: &ProvidersModel, into: &gtk::gio::File, key: Option<&str>) -> Result<()>;
}

mod aegis;
mod andotp;
mod bitwarden;
mod freeotp;
mod google;
mod legacy;
pub use self::{
    aegis::Aegis, andotp::AndOTP, bitwarden::Bitwarden, freeotp::FreeOTP,
    google::Google, legacy::LegacyAuthenticator,
};
