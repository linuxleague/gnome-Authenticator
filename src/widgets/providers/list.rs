use crate::{
    config,
    models::{Account, Provider, ProviderSorter, ProvidersModel},
    widgets::providers::ProviderRow,
};
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, prelude::*, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass;

    #[derive(Debug, CompositeTemplate)]
    pub struct ProvidersList {
        pub filter_model: gtk::FilterListModel,
        pub sorter: ProviderSorter,
        #[template_child]
        pub providers_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub empty_img: TemplateChild<gtk::Image>,
    }

    impl ObjectSubclass for ProvidersList {
        const NAME: &'static str = "ProvidersList";
        type Type = super::ProvidersList;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let filter_model = gtk::FilterListModel::new(gio::NONE_LIST_MODEL, gtk::NONE_FILTER);
            Self {
                providers_list: TemplateChild::default(),
                empty_img: TemplateChild::default(),
                sorter: ProviderSorter::new(),
                filter_model,
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
            Self::bind_template_children(klass);
            klass.add_signal(
                "shared",
                glib::SignalFlags::ACTION,
                &[Account::static_type()],
                glib::Type::Unit,
            );
        }
    }

    impl ObjectImpl for ProvidersList {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProvidersList {}
    impl BoxImpl for ProvidersList {}
}

glib::wrapper! {
    pub struct ProvidersList(ObjectSubclass<imp::ProvidersList>) @extends gtk::Widget, gtk::Box;
}
impl ProvidersList {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProvidersList")
    }

    pub fn set_model(&self, model: ProvidersModel) {
        let self_ = imp::ProvidersList::from_instance(self);
        let accounts_filter = gtk::CustomFilter::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        });
        self_.filter_model.set_filter(Some(&accounts_filter));
        self_.filter_model.set_model(Some(&model));
    }

    pub fn refilter(&self) {
        let self_ = imp::ProvidersList::from_instance(self);

        if let Some(filter) = self_.filter_model.get_filter() {
            filter.changed(gtk::FilterChange::Different);
        }
        self_.sorter.changed(gtk::SorterChange::Different);
    }

    pub fn search(&self, text: String) {
        let self_ = imp::ProvidersList::from_instance(self);

        let accounts_filter = gtk::CustomFilter::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.filter(text.clone());
            provider.accounts().get_n_items() != 0
        });
        self_.filter_model.set_filter(Some(&accounts_filter));
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProvidersList::from_instance(self);

        self_
            .empty_img
            .get()
            .set_from_icon_name(Some(config::APP_ID));

        let sort_model = gtk::SortListModel::new(Some(&self_.filter_model), Some(&self_.sorter));

        self_.providers_list.get().bind_model(
            Some(&sort_model),
            clone!(@weak self as list => move |obj| {
                let provider = obj.clone().downcast::<Provider>().unwrap();
                let row = ProviderRow::new(provider);
                row.connect_local("changed", false, clone!(@weak list =>  move |_| {
                    list.refilter();
                    None
                })).unwrap();
                row.connect_local("shared", false, clone!(@weak list =>  move |args| {
                    let account = args.get(1).unwrap().get::<Account>().unwrap().unwrap();

                    list.emit("shared", &[&account]);
                    None
                })).unwrap();

                row.upcast::<gtk::Widget>()
            }),
        );
    }
}
