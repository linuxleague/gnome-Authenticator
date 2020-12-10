use super::account::Account;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::StaticType;
use glib::{glib_object_subclass, glib_wrapper};

mod imp {
    use super::*;
    use gio::subclass::ListModelImpl;
    use glib::subclass;
    use glib::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Debug)]
    pub struct AccountsModel(pub RefCell<Vec<Account>>);

    impl ObjectSubclass for AccountsModel {
        const NAME: &'static str = "AccountsModel";
        type Type = super::AccountsModel;
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
    impl ObjectImpl for AccountsModel {}
    impl ListModelImpl for AccountsModel {
        fn get_item_type(&self, _list_model: &Self::Type) -> glib::Type {
            Account::static_type()
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
    pub struct AccountsModel(ObjectSubclass<imp::AccountsModel>) @implements gio::ListModel;
}

impl AccountsModel {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create AccountsModel")
            .downcast()
            .expect("Created AccountsModel is of wrong type")
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

    pub fn find_by_id(&self, id: i32) -> Option<u32> {
        for pos in 0..self.get_n_items() {
            let obj = self.get_object(pos)?;
            let account = obj.downcast::<Account>().unwrap();
            if account.id() == id {
                return Some(pos);
            }
        }
        None
    }
}
