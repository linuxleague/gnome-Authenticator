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
mod search_provider;
mod settings;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::default);
pub static FAVICONS_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    gtk::glib::user_cache_dir()
        .join("authenticator")
        .join("favicons")
});

pub use self::{
    account::Account,
    account_sorter::AccountSorter,
    accounts::AccountsModel,
    algorithm::{Algorithm, Method},
    keyring::SECRET_SERVICE,
    otp_uri::OTPUri,
    provider::{DieselProvider, Provider, ProviderPatch},
    provider_sorter::ProviderSorter,
    providers::ProvidersModel,
    search_provider::{start, SearchProviderAction},
    settings::Settings,
};
