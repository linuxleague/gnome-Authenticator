use crate::config;
use crate::models::{Account, Provider, ProviderSorter, ProvidersModel};
use crate::widgets::{providers::ProviderRow, Action, View};
use gio::{subclass::ObjectSubclass, ListModelExt};
use glib::subclass::prelude::*;
use glib::Sender;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use once_cell::sync::OnceCell;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    pub struct ProvidersList {
        pub sender: OnceCell<Sender<Action>>,
        #[template_child(id = "providers_list")]
        pub providers_list: TemplateChild<gtk::ListBox>,
        #[template_child(id = "empty_img")]
        pub empty_img: TemplateChild<gtk::Image>,
        pub filter_model: gtk::FilterListModel,
        pub sorter: ProviderSorter,
    }

    impl ObjectSubclass for ProvidersList {
        const NAME: &'static str = "ProvidersList";
        type Type = super::ProvidersList;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let filter_model = gtk::FilterListModel::new(gio::NONE_LIST_MODEL, gtk::NONE_FILTER);
            Self {
                providers_list: TemplateChild::default(),
                empty_img: TemplateChild::default(),
                sorter: ProviderSorter::new(),
                filter_model,
                sender: OnceCell::new(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for ProvidersList {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProvidersList {}
    impl BoxImpl for ProvidersList {}
}

glib_wrapper! {
    pub struct ProvidersList(ObjectSubclass<imp::ProvidersList>) @extends gtk::Widget, gtk::Box;
}
impl ProvidersList {
    pub fn new(sender: Sender<Action>) -> Self {
        let list: ProvidersList = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProvidersList")
            .downcast::<ProvidersList>()
            .expect("Created object is of wrong type");
        let self_ = imp::ProvidersList::from_instance(&list);
        self_.sender.set(sender).unwrap();
        list.setup_widgets();
        list
    }

    pub fn set_model(&self, model: ProvidersModel) {
        let self_ = imp::ProvidersList::from_instance(self);
        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(|object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        })));
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

        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.filter(text.clone());
            provider.accounts().get_n_items() != 0
        })));
        self_.filter_model.set_filter(Some(&accounts_filter));
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProvidersList::from_instance(self);

        self_
            .empty_img
            .get()
            .set_from_icon_name(Some(config::APP_ID));

        let sort_model = gtk::SortListModel::new(Some(&self_.filter_model), Some(&self_.sorter));
        let sender = self_.sender.get().unwrap();
        self_.providers_list.get().bind_model(
            Some(&sort_model),
            Some(Box::new(
                clone!(@weak self as list, @strong sender => move |obj| {
                    let provider = obj.clone().downcast::<Provider>().unwrap();
                    let row = ProviderRow::new(provider);
                    row.connect_local("changed", false, clone!(@weak list =>  move |_| {
                        list.refilter();
                        None
                    })).unwrap();
                    row.connect_local("shared", false, clone!(@strong sender =>  move |args| {
                        let account = args.get(1).unwrap().get::<Account>().unwrap().unwrap();
                        send!(sender, Action::SetView(View::Account(account)));
                        None
                    })).unwrap();

                    row.upcast::<gtk::Widget>()
                }),
            )),
        );
    }
}
