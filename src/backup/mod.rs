use std::fmt::Debug;

use crate::models::{Account, Algorithm, OTPMethod, ProvidersModel};
use anyhow::Result;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Operation {
    Backup,
    Restore,
}

pub trait Restorable: Sized {
    type Item: RestorableItem;

    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `restore.$identifier` action
    fn identifier() -> String;
    fn restore(from: &gtk::gio::File) -> Result<Vec<Self::Item>>;
    fn restore_item(item: &Self::Item, model: &ProvidersModel) -> Result<()> {
        let provider = model.find_or_create(
            &item.issuer(),
            item.period(),
            item.method(),
            None,
            item.algorithm(),
            item.digits(),
            item.counter(),
            None,
            None,
        )?;

        let account = Account::create(&item.account(), &item.secret(), &provider)?;
        provider.add_account(&account);
        Ok(())
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
