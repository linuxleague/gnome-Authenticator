use super::{Account, Algorithm, OTPMethod, Provider};
use anyhow::Result;
use gio::{prelude::*, subclass::ObjectSubclass};
use glib::{glib_object_subclass, glib_wrapper, StaticType};
use gtk::prelude::*;

mod imp {
    use super::*;
    use gio::subclass::ListModelImpl;
    use glib::{subclass, subclass::prelude::*};
    use std::cell::RefCell;

    #[derive(Debug)]
    pub struct ProvidersModel(pub RefCell<Vec<Provider>>);

    impl ObjectSubclass for ProvidersModel {
        const NAME: &'static str = "ProvidersModel";
        type Type = super::ProvidersModel;
        type ParentType = glib::Object;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn type_init(type_: &mut subclass::InitializingType<Self>) {
            type_.add_interface::<gio::ListModel>();
        }

        fn new() -> Self {
            Self(RefCell::new(Vec::new()))
        }
    }
    impl ObjectImpl for ProvidersModel {}
    impl ListModelImpl for ProvidersModel {
        fn get_item_type(&self, _list_model: &Self::Type) -> glib::Type {
            Provider::static_type()
        }
        fn get_n_items(&self, _list_model: &Self::Type) -> u32 {
            self.0.borrow().len() as u32
        }
        fn get_item(&self, _list_model: &Self::Type, position: u32) -> Option<glib::Object> {
            self.0
                .borrow()
                .get(position as usize)
                .map(|o| o.clone().upcast::<glib::Object>())
        }
    }
}

glib_wrapper! {
    pub struct ProvidersModel(ObjectSubclass<imp::ProvidersModel>) @implements gio::ListModel;
}

impl ProvidersModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let model: ProvidersModel = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create Model")
            .downcast()
            .expect("Created Model is of wrong type");
        model.init();
        model
    }

    pub fn find_or_create(
        &self,
        name: &str,
        period: i32,
        method: OTPMethod,
        website: Option<String>,
        algorithm: Algorithm,
        digits: i32,
        default_counter: i32,
    ) -> Result<Provider> {
        let provider = match self.find_by_name(name) {
            Some(p) => p,
            None => {
                let p = Provider::create(
                    name,
                    period,
                    algorithm,
                    website,
                    method,
                    digits,
                    default_counter,
                )?;
                self.add_provider(&p);
                p
            }
        };
        Ok(provider)
    }

    pub fn find_by_name(&self, name: &str) -> Option<Provider> {
        for pos in 0..self.get_n_items() {
            let obj = self.get_object(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.name() == name {
                return Some(provider);
            }
        }
        None
    }

    pub fn find_by_id(&self, id: i32) -> Option<Provider> {
        for pos in 0..self.get_n_items() {
            let obj = self.get_object(pos)?;
            let provider = obj.downcast::<Provider>().unwrap();
            if provider.id() == id {
                return Some(provider);
            }
        }
        None
    }

    pub fn completion_model(&self) -> gtk::ListStore {
        let store = gtk::ListStore::new(&[i32::static_type(), String::static_type()]);
        for pos in 0..self.get_n_items() {
            let obj = self.get_object(pos).unwrap();
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
        let self_ = imp::ProvidersModel::from_instance(self);
        let pos = {
            let mut data = self_.0.borrow_mut();
            data.push(provider.clone());
            (data.len() - 1) as u32
        };
        self.items_changed(pos, 0, 1);
    }

    pub fn add_account(&self, account: &Account, provider: &Provider) -> Result<()> {
        let mut found = false;
        for pos in 0..self.get_n_items() {
            let obj = self.get_object(pos).unwrap();
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
        Ok(())
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
