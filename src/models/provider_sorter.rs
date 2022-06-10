use gtk::glib;

use super::provider::Provider;

mod imp {
    use gtk::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ProviderSorter;

    #[glib::object_subclass]
    impl ObjectSubclass for ProviderSorter {
        const NAME: &'static str = "ProviderSorter";
        type Type = super::ProviderSorter;
        type ParentType = gtk::Sorter;
    }

    impl ObjectImpl for ProviderSorter {}
    impl SorterImpl for ProviderSorter {
        fn order(&self, _sorter: &Self::Type) -> gtk::SorterOrder {
            gtk::SorterOrder::Total
        }

        fn compare(
            &self,
            _sorter: &Self::Type,
            item1: &glib::Object,
            item2: &glib::Object,
        ) -> gtk::Ordering {
            Provider::compare(item1, item2).into()
        }
    }
}

glib::wrapper! {
    pub struct ProviderSorter(ObjectSubclass<imp::ProviderSorter>) @extends gtk::Sorter;
}

impl Default for ProviderSorter {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderSorter")
    }
}
