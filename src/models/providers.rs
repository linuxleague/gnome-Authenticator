use super::accounts::AccountsModel;
use super::database;
use super::provider::Provider;
use gio::prelude::*;
use glib::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct ProvidersModel {
    pub model: HashMap<Provider, AccountsModel>,
}

impl ProvidersModel {
    pub fn new() -> Self {
        let mut model = Self { model: HashMap::new() };
        model.init();
        model
    }

    pub fn add_provider(&mut self, provider: Provider) {
        let accounts_model = AccountsModel::from_provider(&provider);
        self.model.insert(provider, accounts_model);
    }

    pub fn get_count(&self) -> usize {
        self.model.len()
    }

    fn init(&mut self) {
        // fill in the providers from the database
        let providers = database::get_providers().unwrap();

        for provider in providers.into_iter() {
            self.add_provider(provider);
        }
    }
}
