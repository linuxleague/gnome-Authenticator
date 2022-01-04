use super::ProviderPage;
use crate::models::{Provider, ProviderSorter, ProvidersModel};
use adw::{prelude::*, subclass::prelude::*};
use glib::clone;
use gtk::{gio, glib, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;
use row::ProviderActionRow;

enum View {
    List,
    Form,
}

mod imp {
    use super::*;
    use adw::subclass::window::AdwWindowImpl;
    use glib::subclass::{self, Signal};

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/providers_dialog.ui")]
    pub struct ProvidersDialog {
        pub page: ProviderPage,
        pub actions: gio::SimpleActionGroup,
        pub filter_model: gtk::FilterListModel,
        #[template_child]
        pub providers_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub deck: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub title_stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProvidersDialog {
        const NAME: &'static str = "ProvidersDialog";
        type Type = super::ProvidersDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProvidersDialog {
        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("changed", &[], <()>::static_type().into()).build()]
            });
            SIGNALS.as_ref()
        }
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProvidersDialog {}
    impl WindowImpl for ProvidersDialog {}
    impl AdwWindowImpl for ProvidersDialog {}
}
glib::wrapper! {
    pub struct ProvidersDialog(ObjectSubclass<imp::ProvidersDialog>) @extends gtk::Widget, gtk::Window, adw::Window;
}

impl ProvidersDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog =
            glib::Object::new::<ProvidersDialog>(&[]).expect("Failed to create ProvidersDialog");

        dialog.setup_widgets(model);
        dialog.setup_actions();
        dialog
    }

    fn setup_widgets(&self, model: ProvidersModel) {
        let imp = self.imp();

        imp.filter_model.set_model(Some(&model));

        let stack = &*imp.stack;
        imp.filter_model
            .connect_items_changed(clone!(@weak stack => move |model, _, _, _| {
                if model.n_items() == 0 {
                    stack.set_visible_child_name("no-results");
                } else {
                    stack.set_visible_child_name("results");
                }
            }));

        let search_entry = &*imp.search_entry;
        search_entry.connect_search_changed(clone!(@weak self as dialog => move |entry| {
            let text = entry.text().to_string();
            dialog.search(text);
        }));

        let search_btn = &*imp.search_btn;
        search_entry.connect_search_started(clone!(@weak search_btn => move |_| {
            search_btn.set_active(true);
        }));
        search_entry.connect_stop_search(clone!(@weak search_btn => move |_| {
            search_btn.set_active(false);
        }));

        let title_stack = &*imp.title_stack;
        search_btn.connect_toggled(clone!(@weak title_stack, @weak search_entry => move |btn| {
            if btn.is_active() {
                title_stack.set_visible_child_name("search");
                search_entry.grab_focus();
            } else {
                search_entry.set_text("");
                title_stack.set_visible_child_name("title");
            }
        }));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, list_item| {
            let row = ProviderActionRow::new();
            list_item.set_child(Some(&row));
        });
        factory.connect_bind(|_, list_item| {
            let row = list_item
                .child()
                .unwrap()
                .downcast::<ProviderActionRow>()
                .unwrap();
            let item = list_item.item().unwrap();
            let provider = item.downcast::<Provider>().unwrap();
            row.set_provider(provider);
        });

        imp.providers_list.set_factory(Some(&factory));
        let sorter = ProviderSorter::default();
        let sort_model = gtk::SortListModel::new(Some(&imp.filter_model), Some(&sorter));

        let selection_model = gtk::NoSelection::new(Some(&sort_model));
        imp.providers_list.set_model(Some(&selection_model));

        imp.providers_list
            .connect_activate(clone!(@weak self as dialog => move |listview, pos| {
                let model = listview.model().unwrap();
                let provider = model
                    .item(pos)
                    .unwrap()
                    .downcast::<Provider>()
                    .unwrap();
                dialog.edit_provider(provider);
            }));

        imp.page.connect_local(
            "created",
            false,
            clone!(@weak model, @weak self as dialog => @default-return None, move |args| {
                let provider = args.get(1).unwrap().get::<Provider>().unwrap();
                model.add_provider(&provider);
                dialog.set_view(View::List);
                dialog.emit_by_name::<()>("changed", &[]);
                None
            }),
        );

        imp.page.connect_local(
            "updated",
            false,
            clone!(@weak self as dialog => @default-return None, move |_| {
                dialog.set_view(View::List);
                dialog.emit_by_name::<()>("changed", &[]);
                None
            }),
        );

        imp.page.connect_local(
            "deleted",
            false,
            clone!(@weak model, @weak self as dialog => @default-return None, move |args| {
                let provider = args.get(1).unwrap().get::<Provider>().unwrap();
                model.delete_provider(&provider);
                dialog.set_view(View::List);
                dialog.emit_by_name::<()>("changed", &[]);
                None
            }),
        );
        let deck_page = imp.deck.append(&imp.page);
        deck_page.set_name(Some("provider"));
        self.set_view(View::List);
    }

    fn setup_actions(&self) {
        let imp = self.imp();

        action!(
            imp.actions,
            "back",
            clone!(@weak self as dialog => move |_ , _| {
                dialog.set_view(View::List);
            })
        );

        action!(
            imp.actions,
            "add",
            clone!(@weak self as dialog => move |_, _| {
                dialog.add_provider();
            })
        );

        let search_btn = &*imp.search_btn;
        action!(
            imp.actions,
            "search",
            clone!(@weak search_btn => move |_,_| {
                search_btn.set_active(!search_btn.is_active());
            })
        );

        self.insert_action_group("providers", Some(&imp.actions));
    }

    fn search(&self, text: String) {
        let providers_filter = gtk::CustomFilter::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider
                .name()
                .to_ascii_lowercase()
                .contains(&text.to_ascii_lowercase())
        });
        self.imp().filter_model.set_filter(Some(&providers_filter));
    }

    fn add_provider(&self) {
        self.set_view(View::Form);
        // By not setting the current provider we implicitly say it's for creating a new one
        self.imp().page.set_provider(None);
    }

    fn edit_provider(&self, provider: Provider) {
        self.set_view(View::Form);
        self.imp().page.set_provider(Some(provider));
    }

    fn set_view(&self, view: View) {
        let imp = self.imp();
        match view {
            View::Form => {
                imp.deck.set_visible_child_name("provider");
                imp.search_entry.set_key_capture_widget(gtk::Widget::NONE);
                imp.search_entry.emit_stop_search();
            }
            View::List => {
                imp.deck.set_visible_child_name("providers");
                imp.search_entry.set_key_capture_widget(Some(self));
            }
        }
    }
}

mod row {
    use super::*;
    mod imp {
        use super::*;
        use glib::{ParamSpec, ParamSpecObject, Value};
        use std::cell::RefCell;

        pub struct ProviderActionRow {
            pub provider: RefCell<Option<Provider>>,
            pub actions: gio::SimpleActionGroup,
            pub title_label: gtk::Label,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for ProviderActionRow {
            const NAME: &'static str = "ProviderActionRow";
            type Type = super::ProviderActionRow;
            type ParentType = adw::Bin;

            fn new() -> Self {
                let actions = gio::SimpleActionGroup::new();
                let title_label = gtk::Label::new(None);

                Self {
                    actions,
                    title_label,
                    provider: RefCell::new(None),
                }
            }
        }

        impl ObjectImpl for ProviderActionRow {
            fn properties() -> &'static [ParamSpec] {
                use once_cell::sync::Lazy;
                static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                    vec![ParamSpecObject::new(
                        "provider",
                        "Provider",
                        "The Provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE,
                    )]
                });
                PROPERTIES.as_ref()
            }

            fn set_property(
                &self,
                _obj: &Self::Type,
                _id: usize,
                value: &Value,
                pspec: &ParamSpec,
            ) {
                match pspec.name() {
                    "provider" => {
                        let provider = value.get().unwrap();
                        self.provider.replace(provider);
                    }
                    _ => unimplemented!(),
                }
            }

            fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
                match pspec.name() {
                    "provider" => self.provider.borrow().to_value(),
                    _ => unimplemented!(),
                }
            }

            fn constructed(&self, obj: &Self::Type) {
                obj.setup_widgets();
                self.parent_constructed(obj);
            }
        }
        impl WidgetImpl for ProviderActionRow {}
        impl BinImpl for ProviderActionRow {}
    }

    glib::wrapper! {
        pub struct ProviderActionRow(ObjectSubclass<imp::ProviderActionRow>) @extends gtk::Widget, adw::Bin;
    }

    impl ProviderActionRow {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            glib::Object::new(&[]).expect("Failed to create ProviderActionRow")
        }

        fn setup_widgets(&self) {
            let imp = self.imp();
            let hbox = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .margin_bottom(16)
                .margin_end(16)
                .margin_top(16)
                .margin_start(16)
                .build();
            imp.title_label.set_valign(gtk::Align::Center);
            imp.title_label.set_halign(gtk::Align::Start);
            hbox.append(&imp.title_label);
            self.set_child(Some(&hbox));
        }

        pub fn set_provider(&self, provider: Provider) {
            self.set_property("provider", &provider);
            self.imp().title_label.set_text(&provider.name());
        }

        pub fn provider(&self) -> Option<Provider> {
            self.property("provider")
        }
    }
}
