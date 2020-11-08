use super::account::Account;
use super::provider::Provider;
use gio::prelude::*;
use glib::StaticType;
use gtk::prelude::*;

pub struct ProvidersModel {
    pub model: gio::ListStore,
}

impl ProvidersModel {
    pub fn new() -> Self {
        let model = Self {
            model: gio::ListStore::new(Provider::static_type()),
        };
        model.init();
        model
    }

    pub fn find_by_name(&self, name: &str) -> Option<Provider> {
        for pos in 0..self.count() {
            let obj = self.model.get_object(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.name() == name {
                return Some(provider);
            }
        }
        None
    }

    pub fn find_by_id(&self, id: i32) -> Option<Provider> {
        for pos in 0..self.count() {
            let obj = self.model.get_object(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.id() == id {
                return Some(provider);
            }
        }
        None
    }

    pub fn completion_model(&self) -> gtk::ListStore {
        let store = gtk::ListStore::new(&[i32::static_type(), String::static_type()]);
        for pos in 0..self.count() {
            let obj = self.model.get_object(pos).unwrap();
            let provider = obj.downcast_ref::<Provider>().unwrap();
            store.set(
                &store.append(),
                &[0, 1],
                &[&provider.id(), &provider.name()],
            );
        }
        store
    }

    pub fn add_provider(&self, provider: &Provider) {
        self.model.append(provider);
    }

    pub fn add_account(&self, account: &Account, provider: &Provider) {
        let mut found = false;
        for pos in 0..self.count() {
            let obj = self.model.get_object(pos).unwrap();
            let p = obj.downcast_ref::<Provider>().unwrap();
            if p.id() == provider.id() {
                found = true;
                p.add_account(account);
                break;
            }
        }
        if !found {
            provider.add_account(account);
            self.add_provider(provider);
        }
    }

    pub fn count(&self) -> u32 {
        self.model.get_n_items()
    }

    fn init(&self) {
        // fill in the providers from the database
        let providers = Provider::load().unwrap();

        for provider in providers.iter() {
            self.add_provider(provider);
        }
    }
}
