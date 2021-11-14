use super::account::Account;
use glib::StaticType;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct AccountsModel(pub RefCell<Vec<Account>>);

    #[glib::object_subclass]
    impl ObjectSubclass for AccountsModel {
        const NAME: &'static str = "AccountsModel";
        type Type = super::AccountsModel;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for AccountsModel {}
    impl ListModelImpl for AccountsModel {
        fn item_type(&self, _list_model: &Self::Type) -> glib::Type {
            Account::static_type()
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
    pub struct AccountsModel(ObjectSubclass<imp::AccountsModel>) @implements gio::ListModel;
}

impl AccountsModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create AccountsModel")
    }

    pub fn insert(&self, account: &Account) {
        let self_ = imp::AccountsModel::from_instance(self);
        let pos = {
            let mut data = self_.0.borrow_mut();
            data.push(account.clone());
            (data.len() - 1) as u32
        };
        self.items_changed(pos, 0, 1);
    }

    pub fn remove(&self, pos: u32) {
        let self_ = imp::AccountsModel::from_instance(self);
        self_.0.borrow_mut().remove(pos as usize);
        self.items_changed(pos, 1, 0);
    }

    pub fn find_by_id(&self, id: u32) -> Option<u32> {
        for pos in 0..self.n_items() {
            let obj = self.item(pos)?;
            let account = obj.downcast::<Account>().unwrap();
            if account.id() == id {
                return Some(pos);
            }
        }
        None
    }
}
