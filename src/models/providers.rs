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

    pub fn find_by_id(&self, id: i32) -> Option<Provider> {
        for pos in 0..self.count() {
            let obj = self.model.get_object(pos).unwrap();
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
        //let accounts_model = AccountsModel::from_provider(&provider);
        self.model.append(provider);
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
