use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use glib::clone;
use gtk::{glib, pango, subclass::prelude::*, CompositeTemplate};
use row::ProviderActionRow;

use super::ProviderPage;
use crate::models::{Provider, ProviderSorter, ProvidersModel};

enum View {
    List,
    Form,
    Placeholder,
}

mod imp {
    use adw::subclass::window::AdwWindowImpl;
    use glib::subclass::{self, Signal};

    use super::*;
    use crate::config;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/providers_dialog.ui")]
    pub struct ProvidersDialog {
        #[template_child]
        pub page: TemplateChild<ProviderPage>,
        pub filter_model: gtk::FilterListModel,
        #[template_child]
        pub providers_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub deck: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub placeholder_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProvidersDialog {
        const NAME: &'static str = "ProvidersDialog";
        type Type = super::ProvidersDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("providers.back", None, move |dialog, _, _| {
                dialog.set_view(View::List);
            });

            klass.install_action("providers.add", None, move |dialog, _, _| {
                dialog.add_provider();
            });

            klass.install_action("providers.search", None, move |dialog, _, _| {
                let search_btn = &*dialog.imp().search_btn;
                search_btn.set_active(!search_btn.is_active());
            });
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
            self.parent_constructed(obj);
            self.placeholder_page.set_icon_name(Some(config::APP_ID));
        }
    }
    impl WidgetImpl for ProvidersDialog {}
    impl WindowImpl for ProvidersDialog {}
    impl AdwWindowImpl for ProvidersDialog {}
}
glib::wrapper! {
    pub struct ProvidersDialog(ObjectSubclass<imp::ProvidersDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl ProvidersDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog =
            glib::Object::new::<ProvidersDialog>(&[]).expect("Failed to create ProvidersDialog");

        dialog.setup_widgets(model);
        dialog
    }

    fn setup_widgets(&self, model: ProvidersModel) {
        let imp = self.imp();

        imp.filter_model.set_model(Some(&model));

        let stack = &*imp.search_stack;
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

        let sorter = ProviderSorter::default();
        let sort_model = gtk::SortListModel::new(Some(&imp.filter_model), Some(&sorter));

        let selection_model = gtk::NoSelection::new(Some(&sort_model));
        imp.providers_list
            .bind_model(Some(&selection_model), move |obj| {
                let provider = obj.clone().downcast::<Provider>().unwrap();
                let row = ProviderActionRow::default();
                row.set_provider(provider);
                row.upcast::<gtk::Widget>()
            });

        imp.providers_list.connect_row_activated(
            clone!(@weak self as dialog => move |_list, row| {
                let row = row.downcast_ref::<ProviderActionRow>().unwrap();
                let provider = row.provider();
                dialog.edit_provider(provider);
            }),
        );

        imp.page.connect_local(
            "created",
            false,
            clone!(@weak model, @weak self as dialog => @default-return None, move |args| {
                let provider = args[1].get::<Provider>().unwrap();
                model.append(&provider);
                dialog.emit_by_name::<()>("changed", &[]);
                dialog.imp().toast_overlay.add_toast(&adw::Toast::new(&gettext("Provider created successfully")));
                dialog.set_view(View::Placeholder);
                None
            }),
        );

        imp.page.connect_local(
            "updated",
            false,
            clone!(@weak self as dialog => @default-return None, move |_| {
                dialog.set_view(View::List);
                dialog.emit_by_name::<()>("changed", &[]);
                dialog.imp().toast_overlay.add_toast(&adw::Toast::new(&gettext("Provider updated successfully")));
                None
            }),
        );

        imp.page.connect_local(
            "deleted",
            false,
            clone!(@weak model, @weak self as dialog => @default-return None, move |args| {
                let provider = args[1].get::<Provider>().unwrap();
                model.delete_provider(&provider);
                dialog.set_view(View::Placeholder);
                dialog.emit_by_name::<()>("changed", &[]);
                dialog.imp().toast_overlay.add_toast(&adw::Toast::new(&gettext("Provider removed successfully")));
                None
            }),
        );

        imp.deck
            .bind_property("folded", &*imp.page.imp().revealer, "reveal-child")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        self.set_view(View::Placeholder);
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
        // By not setting the current provider we implicitly say it's for creating a new
        // one
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
                imp.stack.set_visible_child_name("provider");
                imp.search_entry.set_key_capture_widget(gtk::Widget::NONE);
                imp.search_entry.emit_stop_search();
            }
            View::List => {
                imp.deck.set_visible_child_name("providers");
                imp.search_entry.set_key_capture_widget(Some(self));
            }
            View::Placeholder => {
                imp.deck.set_visible_child_name("provider");
                imp.stack.set_visible_child_name("placeholder");
                imp.search_entry.set_key_capture_widget(Some(self));
            }
        }
    }
}

mod row {
    use super::*;
    mod imp {
        use std::cell::RefCell;

        use glib::{ParamFlags, ParamSpec, ParamSpecObject, Value};
        use once_cell::sync::Lazy;

        use super::*;

        #[derive(Debug, Default)]
        pub struct ProviderActionRow {
            pub provider: RefCell<Option<Provider>>,
            pub title_label: gtk::Label,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for ProviderActionRow {
            const NAME: &'static str = "ProviderActionRow";
            type Type = super::ProviderActionRow;
            type ParentType = gtk::ListBoxRow;
        }

        impl ObjectImpl for ProviderActionRow {
            fn properties() -> &'static [ParamSpec] {
                static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                    vec![ParamSpecObject::new(
                        "provider",
                        "",
                        "",
                        Provider::static_type(),
                        ParamFlags::READWRITE,
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
                self.parent_constructed(obj);
                let hbox = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .margin_bottom(12)
                    .margin_end(6)
                    .margin_top(12)
                    .margin_start(6)
                    .build();
                self.title_label.set_valign(gtk::Align::Center);
                self.title_label.set_halign(gtk::Align::Start);
                self.title_label.set_wrap(true);
                self.title_label.set_ellipsize(pango::EllipsizeMode::End);
                hbox.append(&self.title_label);
                obj.set_child(Some(&hbox));
            }
        }
        impl WidgetImpl for ProviderActionRow {}
        impl ListBoxRowImpl for ProviderActionRow {}
    }

    glib::wrapper! {
        pub struct ProviderActionRow(ObjectSubclass<imp::ProviderActionRow>)
            @extends gtk::Widget, gtk::ListBoxRow;
    }

    impl ProviderActionRow {
        pub fn set_provider(&self, provider: Provider) {
            self.set_property("provider", &provider);
            self.imp().title_label.set_text(&provider.name());
        }

        pub fn provider(&self) -> Provider {
            self.property("provider")
        }
    }

    impl Default for ProviderActionRow {
        fn default() -> Self {
            glib::Object::new(&[]).expect("Failed to create ProviderActionRow")
        }
    }
}
