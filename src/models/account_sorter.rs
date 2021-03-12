use super::account::Account;
use gtk::glib;

mod imp {
    use super::*;
    use glib::subclass::prelude::*;
    use gtk::subclass::sorter::SorterImpl;

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
        fn get_order(&self, _sorter: &Self::Type) -> gtk::SorterOrder {
            gtk::SorterOrder::Total
        }

        fn compare(
            &self,
            _sorter: &Self::Type,
            item1: &glib::Object,
            item2: &glib::Object,
        ) -> gtk::Ordering {
            Account::compare(item1, item2).into()
        }
    }
}

glib::wrapper! {
    pub struct AccountSorter(ObjectSubclass<imp::AccountSorter>) @extends gtk::Sorter;
}

impl AccountSorter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create AccountSorter")
    }
}
