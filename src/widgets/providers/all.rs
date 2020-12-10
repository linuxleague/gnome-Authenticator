use super::{ProviderPage, ProviderPageMode};
use crate::models::{Provider, ProviderSorter, ProvidersModel};
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use gio::ListModelExt;
use glib::subclass::prelude::*;
use glib::{clone, glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use row::ProviderActionRow;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::window::WindowImpl as HdyWindowImpl;

    #[derive(CompositeTemplate)]
    pub struct ProvidersDialog {
        pub page: ProviderPage,
        pub actions: gio::SimpleActionGroup,
        pub filter_model: gtk::FilterListModel,
        #[template_child]
        pub providers_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub deck: TemplateChild<libhandy::Leaflet>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
    }

    impl ObjectSubclass for ProvidersDialog {
        const NAME: &'static str = "ProvidersDialog";
        type Type = super::ProvidersDialog;
        type ParentType = libhandy::Window;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let filter_model = gtk::FilterListModel::new(gio::NONE_LIST_MODEL, gtk::NONE_FILTER);
            Self {
                deck: TemplateChild::default(),
                providers_list: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                search_bar: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                page: ProviderPage::new(),
                actions: gio::SimpleActionGroup::new(),
                filter_model,
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/providers_all.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for ProvidersDialog {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProvidersDialog {}
    impl WindowImpl for ProvidersDialog {}
    impl HdyWindowImpl for ProvidersDialog {}
}
glib_wrapper! {
    pub struct ProvidersDialog(ObjectSubclass<imp::ProvidersDialog>) @extends gtk::Widget, gtk::Window, libhandy::Window;
}

impl ProvidersDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProvidersDialog")
            .downcast::<ProvidersDialog>()
            .expect("Created object is of wrong type");

        dialog.setup_widgets(model);
        dialog.setup_actions();
        dialog
    }

    fn setup_widgets(&self, model: ProvidersModel) {
        let self_ = imp::ProvidersDialog::from_instance(self);

        self_.filter_model.set_model(Some(&model));
        self_
            .search_bar
            .get()
            .bind_property("search-mode-enabled", &self_.search_btn.get(), "active")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        self_.search_entry.get().connect_search_changed(
            clone!(@weak self as dialog => move |entry| {
                let text = entry.get_text().unwrap().to_string();
                dialog.search(text);
            }),
        );

        self_
            .search_btn
            .get()
            .bind_property("active", &self_.search_bar.get(), "search-mode-enabled")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, list_item| {
            let row = ProviderActionRow::new();
            list_item.set_child(Some(&row));
        });
        factory.connect_bind(|_, list_item| {
            let row = list_item
                .get_child()
                .unwrap()
                .downcast::<ProviderActionRow>()
                .unwrap();
            let item = list_item.get_item().unwrap();
            let provider = item.downcast::<Provider>().unwrap();
            row.set_provider(provider);
        });

        self_.providers_list.get().set_factory(Some(&factory));
        let sorter = ProviderSorter::new();
        let sort_model = gtk::SortListModel::new(Some(&self_.filter_model), Some(&sorter));

        let selection_model = gtk::NoSelection::new(Some(&sort_model));
        self_.providers_list.get().set_model(Some(&selection_model));

        self_.providers_list.get().connect_activate(
            clone!(@weak self as dialog => move |listview, pos| {
                let model = listview.get_model().unwrap();
                let provider = model
                    .get_object(pos)
                    .unwrap()
                    .downcast::<Provider>()
                    .unwrap();
                dialog.edit_provider(provider);
            }),
        );

        let deck_page = self_.deck.get().add(&self_.page).unwrap();
        deck_page.set_name("provider");
    }

    fn setup_actions(&self) {
        let self_ = imp::ProvidersDialog::from_instance(self);

        let deck = self_.deck.get();
        let search_bar = self_.search_bar.get();
        gtk_macros::action!(
            self_.actions,
            "search",
            clone!(@weak search_bar => move |_,_| {
                search_bar.set_search_mode(!search_bar.get_search_mode());
            })
        );
        gtk_macros::action!(
            self_.actions,
            "back",
            clone!(@weak deck => move |_ , _| {
                deck.set_visible_child_name("providers");
            })
        );

        gtk_macros::action!(
            self_.actions,
            "add",
            clone!(@weak self as dialog => move |_, _| {
                dialog.add_provider();
            })
        );

        self.insert_action_group("providers", Some(&self_.actions));
    }

    fn search(&self, text: String) {
        let self_ = imp::ProvidersDialog::from_instance(self);

        let providers_filter = gtk::CustomFilter::new(Some(Box::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider
                .name()
                .to_ascii_lowercase()
                .contains(&text.to_ascii_lowercase())
        })));
        self_.filter_model.set_filter(Some(&providers_filter));
    }

    fn add_provider(&self) {
        let self_ = imp::ProvidersDialog::from_instance(self);
        self_.deck.get().set_visible_child_name("provider");
        self_.page.set_mode(ProviderPageMode::Create);
    }

    fn edit_provider(&self, provider: Provider) {
        let self_ = imp::ProvidersDialog::from_instance(self);
        self_.deck.get().set_visible_child_name("provider");
        self_.page.set_provider(provider);
        self_.page.set_mode(ProviderPageMode::Edit);
    }
}

mod row {
    use super::*;
    mod imp {
        use super::*;
        use glib::subclass;
        use gtk::subclass::prelude::*;
        use std::cell::RefCell;

        static PROPERTIES: [subclass::Property; 1] = [subclass::Property("provider", |name| {
            glib::ParamSpec::object(
                name,
                "Provider",
                "The Provider",
                Provider::static_type(),
                glib::ParamFlags::READWRITE,
            )
        })];

        pub struct ProviderActionRow {
            pub provider: RefCell<Option<Provider>>,
            pub actions: gio::SimpleActionGroup,
            pub title_label: gtk::Label,
        }

        impl ObjectSubclass for ProviderActionRow {
            const NAME: &'static str = "ProviderActionRow";
            type Type = super::ProviderActionRow;
            type ParentType = gtk::ListBoxRow;
            type Instance = subclass::simple::InstanceStruct<Self>;
            type Class = subclass::simple::ClassStruct<Self>;

            glib_object_subclass!();

            fn new() -> Self {
                let actions = gio::SimpleActionGroup::new();
                let title_label = gtk::Label::new(None);

                Self {
                    actions,
                    title_label,
                    provider: RefCell::new(None),
                }
            }

            fn class_init(klass: &mut Self::Class) {
                klass.install_properties(&PROPERTIES);
            }
        }

        impl ObjectImpl for ProviderActionRow {
            fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
                let prop = &PROPERTIES[id];

                match *prop {
                    subclass::Property("provider", ..) => {
                        let provider = value
                            .get()
                            .expect("type conformity checked by `Object::set_property`");
                        self.provider.replace(provider);
                    }
                    _ => unimplemented!(),
                }
            }

            fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
                let prop = &PROPERTIES[id];
                match *prop {
                    subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                    _ => unimplemented!(),
                }
            }

            fn constructed(&self, obj: &Self::Type) {
                obj.setup_widgets();
                self.parent_constructed(obj);
            }
        }
        impl WidgetImpl for ProviderActionRow {}
        impl ListBoxRowImpl for ProviderActionRow {}
    }

    glib_wrapper! {
        pub struct ProviderActionRow(ObjectSubclass<imp::ProviderActionRow>) @extends gtk::Widget, gtk::ListBoxRow;
    }

    impl ProviderActionRow {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            glib::Object::new(Self::static_type(), &[])
                .expect("Failed to create ProviderActionRow")
                .downcast::<ProviderActionRow>()
                .expect("Created object is of wrong type")
        }

        fn setup_widgets(&self) {
            let self_ = imp::ProviderActionRow::from_instance(self);
            let hbox = gtk::BoxBuilder::new()
                .orientation(gtk::Orientation::Horizontal)
                .margin_bottom(16)
                .margin_end(16)
                .margin_top(16)
                .margin_start(16)
                .build();
            self_.title_label.set_valign(gtk::Align::Center);
            self_.title_label.set_halign(gtk::Align::Start);
            hbox.append(&self_.title_label);
            self.set_child(Some(&hbox));
        }

        pub fn set_provider(&self, provider: Provider) {
            let self_ = imp::ProviderActionRow::from_instance(self);

            self.set_property("provider", &provider).unwrap();
            self_.title_label.set_text(&provider.name());
        }

        pub fn provider(&self) -> Option<Provider> {
            let provider = self.get_property("provider").unwrap();
            provider.get::<Provider>().unwrap()
        }
    }
}
