use glib::clone;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};

use crate::{
    models::{Account, Provider, ProviderSorter, ProvidersModel},
    widgets::providers::ProviderRow,
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProvidersListView {
    NoSearchResults,
    List,
}

mod imp {
    use glib::subclass::{self, Signal};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/providers_list.ui")]
    pub struct ProvidersList {
        pub filter_model: gtk::FilterListModel,
        pub sorter: ProviderSorter,
        #[template_child]
        pub providers_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProvidersList {
        const NAME: &'static str = "ProvidersList";
        type Type = super::ProvidersList;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProvidersList {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_widget();
        }

        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "shared",
                    &[Account::static_type().into()],
                    <()>::static_type().into(),
                )
                .action()
                .build()]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for ProvidersList {}
    impl BoxImpl for ProvidersList {}
}

glib::wrapper! {
    pub struct ProvidersList(ObjectSubclass<imp::ProvidersList>)
        @extends gtk::Widget, gtk::Box;
}
impl ProvidersList {
    pub fn set_view(&self, view: ProvidersListView) {
        let imp = self.imp();
        match view {
            ProvidersListView::NoSearchResults => {
                imp.stack.set_visible_child_name("no-results");
            }
            ProvidersListView::List => {
                imp.stack.set_visible_child_name("results");
            }
        }
    }

    /// Initialize the ProvidersList by setting the model to use.
    ///
    /// The model contains initially all the providers and are filtered
    /// to keep only the ones that have at least an account.
    pub fn set_model(&self, model: ProvidersModel) {
        let imp = self.imp();
        let accounts_filter = gtk::CustomFilter::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        });
        imp.filter_model.set_filter(Some(&accounts_filter));
        imp.filter_model.set_model(Some(&model));
    }

    pub fn refilter(&self) {
        let imp = self.imp();

        if let Some(filter) = imp.filter_model.filter() {
            filter.changed(gtk::FilterChange::Different);
        }
        imp.sorter.changed(gtk::SorterChange::Different);
    }

    /// Returns an instance of the filtered initial model
    pub fn model(&self) -> gtk::FilterListModel {
        self.imp().filter_model.clone()
    }

    pub fn search(&self, text: String) {
        let accounts_filter = gtk::CustomFilter::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.filter(text.clone());
            provider.accounts().n_items() != 0
        });
        self.imp().filter_model.set_filter(Some(&accounts_filter));
    }

    fn setup_widget(&self) {
        let imp = self.imp();

        let sort_model = gtk::SortListModel::new(Some(&imp.filter_model), Some(&imp.sorter));

        imp.providers_list.bind_model(
            Some(&sort_model),
            clone!(@strong self as list => move |obj| {
                let provider = obj.clone().downcast::<Provider>().unwrap();
                let row = ProviderRow::new(provider);
                row.connect_changed(clone!(@weak list => move |_| {
                    list.refilter();
                }));
                row.connect_shared(clone!(@weak list => move |_, account| {
                    list.emit_by_name::<()>("shared", &[&account]);
                }));

                row.upcast::<gtk::Widget>()
            }),
        );
    }
}
