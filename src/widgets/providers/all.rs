use super::{ProviderPage, ProviderPageMode};
use crate::models::{Provider, ProvidersModel};
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use gio::ListModelExt;
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::{LeafletExt, LeafletPageExt};
use row::ProviderActionRow;
use std::rc::Rc;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::window::WindowImpl as HdyWindowImpl;

    #[derive(CompositeTemplate)]
    pub struct ProvidersDialog {
        #[template_child(id = "providers_list")]
        pub providers_list: TemplateChild<gtk::ListView>,
        #[template_child(id = "deck")]
        pub deck: TemplateChild<libhandy::Leaflet>,
        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child(id = "search_btn")]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        pub page: ProviderPage,
        pub actions: gio::SimpleActionGroup,
        pub filter_model: gtk::FilterListModel,
    }

    impl ObjectSubclass for ProvidersDialog {
        const NAME: &'static str = "ProvidersDialog";
        type Type = super::ProvidersDialog;
        type ParentType = libhandy::Window;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let filter_model =
                gtk::FilterListModel::new(gtk::NONE_FILTER_LIST_MODEL, gtk::NONE_FILTER);
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
    pub fn new(model: Rc<ProvidersModel>) -> Self {
        let dialog = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProvidersDialog")
            .downcast::<ProvidersDialog>()
            .expect("Created object is of wrong type");

        dialog.setup_widgets(model);
        dialog.setup_actions();
        dialog
    }

    fn setup_widgets(&self, model: Rc<ProvidersModel>) {
        let self_ = imp::ProvidersDialog::from_instance(self);

        self_.filter_model.set_model(Some(&model.model));
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
        factory.connect_bind(|_, list_item| {
            let item = list_item.get_item().unwrap();
            let provider = item.clone().downcast::<Provider>().unwrap();
            let row = ProviderActionRow::new(provider);
            list_item.set_child(Some(&row));
        });
        self_.providers_list.get().set_factory(Some(&factory));

        let selection_model = gtk::NoSelection::new(Some(&self_.filter_model));
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
        action!(
            self_.actions,
            "search",
            clone!(@weak search_bar => move |_,_| {
                search_bar.set_search_mode(!search_bar.get_search_mode());
            })
        );
        action!(
            self_.actions,
            "back",
            clone!(@weak deck => move |_ , _| {
                deck.set_visible_child_name("providers");
            })
        );

        action!(
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
                glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
            )
        })];

        pub struct ProviderActionRow {
            pub provider: RefCell<Option<Provider>>,
            pub actions: gio::SimpleActionGroup,
        }

        impl ObjectSubclass for ProviderActionRow {
            const NAME: &'static str = "ProviderActionRow";
            type Type = super::ProviderActionRow;
            type ParentType = libhandy::ActionRow;
            type Instance = subclass::simple::InstanceStruct<Self>;
            type Class = subclass::simple::ClassStruct<Self>;

            glib_object_subclass!();

            fn new() -> Self {
                let actions = gio::SimpleActionGroup::new();

                Self {
                    actions,
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
                obj.init_template();
                obj.setup_widgets();
                self.parent_constructed(obj);
            }
        }
        impl WidgetImpl for ProviderActionRow {}
        impl ListBoxRowImpl for ProviderActionRow {}
        impl libhandy::subclass::action_row::ActionRowImpl for ProviderActionRow {}
    }

    glib_wrapper! {
        pub struct ProviderActionRow(ObjectSubclass<imp::ProviderActionRow>) @extends gtk::Widget, gtk::ListBoxRow, libhandy::ActionRow;
    }

    impl ProviderActionRow {
        pub fn new(provider: Provider) -> Self {
            glib::Object::new(Self::static_type(), &[("provider", &provider)])
                .expect("Failed to create ProviderActionRow")
                .downcast::<ProviderActionRow>()
                .expect("Created object is of wrong type")
        }

        fn provider(&self) -> Provider {
            let provider = self.get_property("provider").unwrap();
            provider.get::<Provider>().unwrap().unwrap()
        }

        fn setup_widgets(&self) {
            self.provider()
                .bind_property("name", self, "title")
                .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
                .build();
        }
    }
}
