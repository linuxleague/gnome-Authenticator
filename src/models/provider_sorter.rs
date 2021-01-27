use super::provider::Provider;
use gtk::glib;

mod imp {
    use super::*;
    use glib::{subclass, subclass::prelude::*};
    use gtk::subclass::sorter::SorterImpl;

    #[derive(Debug)]
    pub struct ProviderSorter;

    impl ObjectSubclass for ProviderSorter {
        const NAME: &'static str = "ProviderSorter";
        type Type = super::ProviderSorter;
        type ParentType = gtk::Sorter;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            Self {}
        }
    }
    impl ObjectImpl for ProviderSorter {}
    impl SorterImpl for ProviderSorter {
        fn get_order(&self, _sorter: &Self::Type) -> gtk::SorterOrder {
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

impl ProviderSorter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderSorter")
    }
}
