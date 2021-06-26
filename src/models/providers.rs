use super::{otp, Account, Algorithm, OTPMethod, Provider};
use anyhow::Result;
use glib::StaticType;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct ProvidersModel(pub RefCell<Vec<Provider>>);

    #[glib::object_subclass]
    impl ObjectSubclass for ProvidersModel {
        const NAME: &'static str = "ProvidersModel";
        type Type = super::ProvidersModel;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }
    impl ObjectImpl for ProvidersModel {}
    impl ListModelImpl for ProvidersModel {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            Provider::static_type()
        }
        fn n_items(&self, _list_model: &Self::Type) -> u32 {
            self.0.borrow().len() as u32
        }
        fn item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
            self.0
                .borrow()
                .get(position as usize)
                .map(|o| o.clone().upcast::<glib::Object>())
        }
    }
}

glib::wrapper! {
    pub struct ProvidersModel(ObjectSubclass<imp::ProvidersModel>) @implements gio::ListModel;
}

impl ProvidersModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let model: ProvidersModel = glib::Object::new(&[]).expect("Failed to create Model");
        model.init();
        model
    }

    #[allow(clippy::too_many_arguments)]
    pub fn find_or_create(
        &self,
        name: &str,
        period: Option<u32>,
        method: OTPMethod,
        website: Option<String>,
        algorithm: Algorithm,
        digits: Option<u32>,
        default_counter: Option<u32>,
        help_url: Option<String>,
        image_uri: Option<String>,
    ) -> Result<Provider> {
        let provider = match self.find_by_name(name) {
            Some(p) => p,
            None => {
                let p = Provider::create(
                    name,
                    period.unwrap_or(otp::TOTP_DEFAULT_PERIOD),
                    algorithm,
                    website,
                    method,
                    digits.unwrap_or(otp::DEFAULT_DIGITS),
                    default_counter.unwrap_or(otp::HOTP_DEFAULT_COUNTER),
                    help_url,
                    image_uri,
                )?;
                self.add_provider(&p);
                p
            }
        };
        Ok(provider)
    }

    pub fn find_by_name(&self, name: &str) -> Option<Provider> {
        for pos in 0..self.n_items() {
            let obj = self.item(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.name() == name {
                return Some(provider);
            }
        }
        None
    }

    pub fn find_by_id(&self, id: u32) -> Option<Provider> {
        for pos in 0..self.n_items() {
            let obj = self.item(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.id() == id {
                return Some(provider);
            }
        }
        None
    }

    pub fn has_providers(&self) -> bool {
        let mut found = false;
        for pos in 0..self.n_items() {
            let obj = self.item(pos).unwrap();
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.has_accounts() {
                found = true;
                break;
            }
        }
        found
    }

    pub fn completion_model(&self) -> gtk::ListStore {
        let store = gtk::ListStore::new(&[u32::static_type(), String::static_type()]);
        for pos in 0..self.n_items() {
            let obj = self.item(pos).unwrap();
            let provider = obj.downcast_ref::<Provider>().unwrap();
            store.set(
                &store.append(),
                &[(0, &provider.id()), (1, &provider.name())],
            );
        }
        store
    }

    pub fn add_provider(&self, provider: &Provider) {
        let self_ = imp::ProvidersModel::from_instance(self);
        let pos = {
            let mut data = self_.0.borrow_mut();
            data.push(provider.clone());
            (data.len() - 1) as u32
        };
        self.items_changed(pos, 0, 1);
    }

    pub fn delete_provider(&self, provider: &Provider) {
        let self_ = imp::ProvidersModel::from_instance(self);
        let mut provider_pos = None;
        for pos in 0..self.n_items() {
            let obj = self.item(pos).unwrap();
            let p = obj.downcast::<Provider>().unwrap();
            if p.id() == provider.id() {
                provider_pos = Some(pos);
                break;
            }
        }
        if let Some(pos) = provider_pos {
            {
                let mut data = self_.0.borrow_mut();
                data.remove(pos as usize);
            }
            self.items_changed(pos, 1, 0);
        }
    }

    pub fn add_account(&self, account: &Account, provider: &Provider) {
        let mut found = false;
        for pos in 0..self.n_items() {
            let obj = self.item(pos).unwrap();
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

    fn init(&self) {
        // fill in the providers from the database
        Provider::load()
            .expect("Failed to load providers from the database")
            .for_each(|provider| {
                self.add_provider(&provider);
            });
    }
}
