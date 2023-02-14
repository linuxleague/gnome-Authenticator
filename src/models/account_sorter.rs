use gtk::glib;

mod imp {
    use gtk::subclass::prelude::*;

    use super::*;
    use crate::models::Account;

    #[derive(Debug, Default)]
    pub struct AccountSorter;

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSorter {
        const NAME: &'static str = "AccountSorter";
        type Type = super::AccountSorter;
        type ParentType = gtk::Sorter;
    }

    impl ObjectImpl for AccountSorter {}
    impl SorterImpl for AccountSorter {
        fn order(&self) -> gtk::SorterOrder {
            gtk::SorterOrder::Total
        }

        fn compare(&self, item1: &glib::Object, item2: &glib::Object) -> gtk::Ordering {
            Account::compare(item1, item2).into()
        }
    }
}

glib::wrapper! {
    pub struct AccountSorter(ObjectSubclass<imp::AccountSorter>)
        @extends gtk::Sorter;
}

impl Default for AccountSorter {
    fn default() -> Self {
        glib::Object::new()
    }
}
