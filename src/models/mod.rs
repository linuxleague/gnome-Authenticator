use once_cell::sync::Lazy;
mod account;
mod account_sorter;
mod accounts;
mod algorithm;
pub mod database;
pub mod i18n;
pub mod keyring;
pub mod otp;
mod otp_uri;
mod provider;
mod provider_sorter;
mod providers;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());
pub static FAVICONS_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    gtk::glib::user_cache_dir()
        .join("authenticator")
        .join("favicons")
});

pub use self::{
    account::Account,
    account_sorter::AccountSorter,
    accounts::AccountsModel,
    algorithm::{Algorithm, OTPMethod},
    keyring::SECRET_SERVICE,
    otp_uri::OTPUri,
    provider::{Provider, ProviderPatch},
    provider_sorter::ProviderSorter,
    providers::ProvidersModel,
};
