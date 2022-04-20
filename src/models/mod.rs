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

pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);
pub static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(100 * 1024 * 1024)
        .build()
        .unwrap()
});

pub use self::{
    account::Account,
    account_sorter::AccountSorter,
    accounts::AccountsModel,
    algorithm::{Algorithm, OTPMethod},
    favicon::{Favicon, FaviconError, FaviconScrapper, Metadata, Type, FAVICONS_PATH},
    keyring::Keyring,
    otp_uri::OTPUri,
    provider::{Provider, ProviderPatch},
    provider_sorter::ProviderSorter,
    providers::ProvidersModel,
};
