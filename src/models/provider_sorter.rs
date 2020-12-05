use super::provider::Provider;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::StaticType;
use glib::{glib_object_subclass, glib_wrapper};

mod imp {
    use super::*;
    use glib::subclass;
    use glib::subclass::prelude::*;
    use gtk::subclass::sorter::SorterImpl;
    use unicase::UniCase;

    #[derive(Debug)]
    pub struct ProviderSorter;

    impl ObjectSubclass for ProviderSorter {
        const NAME: &'static str = "ProviderSorter";
        type Type = super::ProviderSorter;
        type ParentType = gtk::Sorter;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

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
            let provider1 = item1.downcast_ref::<Provider>().unwrap();
            let provider2 = item2.downcast_ref::<Provider>().unwrap();

            UniCase::new(provider1.name())
                .cmp(&UniCase::new(provider2.name()))
                .into()
        }
    }
}

glib_wrapper! {
    pub struct ProviderSorter(ObjectSubclass<imp::ProviderSorter>) @extends gtk::Sorter;
}

impl ProviderSorter {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProviderSorter")
            .downcast()
            .expect("Created ProviderSorter is of wrong type")
    }
}
