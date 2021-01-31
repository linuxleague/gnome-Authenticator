use once_cell::sync::Lazy;
mod account;
mod account_sorter;
mod accounts;
mod algorithm;
pub mod database;
mod favicon;
pub mod i18n;
mod keyring;
pub mod otp;
mod otp_uri;
mod provider;
mod provider_sorter;
mod providers;

pub static CLIENT: Lazy<surf::Client> =
    Lazy::new(|| surf::Client::new().with(surf::middleware::Redirect::default()));

pub use self::{
    account::Account,
    account_sorter::AccountSorter,
    accounts::AccountsModel,
    algorithm::{Algorithm, OTPMethod},
    favicon::{FaviconError, FaviconScrapper, FAVICONS_PATH},
    keyring::Keyring,
    otp_uri::OTPUri,
    provider::{Provider, ProviderPatch},
    provider_sorter::ProviderSorter,
    providers::ProvidersModel,
};
