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
    fn restore(model: ProvidersModel, from: gio::File) -> Result<()>;
}

pub trait Backupable {
    fn title() -> String;
    fn subtitle() -> String;
    // Used to define the `backup.$identifier` action
    fn identifier() -> String;
    fn backup(model: ProvidersModel, into: gio::File) -> Result<()>;
}

mod andotp;
mod bitwarden;
mod freeotp;
mod legacy;
pub use self::andotp::AndOTP;
pub use self::bitwarden::Bitwarden;
pub use self::freeotp::FreeOTP;
pub use self::legacy::LegacyAuthenticator;