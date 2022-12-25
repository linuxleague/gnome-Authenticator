use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use glib::clone;
use gtk::{glib, pango, CompositeTemplate};
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
            klass.bind_template_instance_callbacks();

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
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("changed").build()]);
            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();
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

#[gtk::template_callbacks]
impl ProvidersDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog = glib::Object::new::<ProvidersDialog>(&[]);

        dialog.setup_widget(model);
        dialog
    }

    pub fn connect_changed<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_local(
            "changed",
            false,
            clone!(@weak self as dialog => @default-return None, move |_| {
                callback(&dialog);
                None
            }),
        )
    }

    fn setup_widget(&self, model: ProvidersModel) {
        let imp = self.imp();
        imp.filter_model.set_model(Some(&model));
        imp.filter_model.connect_items_changed(
            clone!(@weak self as dialog => move |model, _, _, _| {
                if model.n_items() == 0 {
                    dialog.imp().search_stack.set_visible_child_name("no-results");
                } else {
                    dialog.imp().search_stack.set_visible_child_name("results");
                }
            }),
        );

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

        imp.deck
            .bind_property("folded", &*imp.page.imp().revealer, "reveal-child")
            .sync_create()
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

    #[template_callback]
    fn on_search_changed(&self, entry: &gtk::SearchEntry) {
        let text = entry.text().to_string();
        self.search(text);
    }

    #[template_callback]
    fn on_search_started(&self, _entry: &gtk::SearchEntry) {
        self.imp().search_btn.set_active(true);
    }
    #[template_callback]
    fn on_search_stopped(&self, _entry: &gtk::SearchEntry) {
        self.imp().search_btn.set_active(false);
    }

    #[template_callback]
    fn on_search_btn_toggled(&self, btn: &gtk::ToggleButton) {
        let imp = self.imp();
        if btn.is_active() {
            imp.title_stack.set_visible_child_name("search");
            imp.search_entry.grab_focus();
        } else {
            imp.search_entry.set_text("");
            imp.title_stack.set_visible_child_name("title");
        }
    }
    #[template_callback]
    fn on_row_activated(&self, row: ProviderActionRow, _list: gtk::ListBox) {
        let provider = row.provider();
        self.edit_provider(provider);
    }

    #[template_callback]
    fn on_provider_created(&self, provider: Provider, _page: ProviderPage) {
        let model = self
            .imp()
            .filter_model
            .model()
            .unwrap()
            .downcast::<ProvidersModel>()
            .unwrap();
        model.append(&provider);
        self.emit_by_name::<()>("changed", &[]);
        self.imp()
            .toast_overlay
            .add_toast(&adw::Toast::new(&gettext("Provider created successfully")));
        self.set_view(View::Placeholder);
    }

    #[template_callback]
    fn on_provider_updated(&self, _provider: Provider, _page: ProviderPage) {
        self.set_view(View::List);
        self.emit_by_name::<()>("changed", &[]);
        self.imp()
            .toast_overlay
            .add_toast(&adw::Toast::new(&gettext("Provider updated successfully")));
    }

    #[template_callback]
    fn on_provider_deleted(&self, provider: Provider, _page: ProviderPage) {
        let model = self
            .imp()
            .filter_model
            .model()
            .unwrap()
            .downcast::<ProvidersModel>()
            .unwrap();
        model.delete_provider(&provider);
        self.set_view(View::Placeholder);
        self.emit_by_name::<()>("changed", &[]);
        self.imp()
            .toast_overlay
            .add_toast(&adw::Toast::new(&gettext("Provider removed successfully")));
    }
}

mod row {
    use super::*;
    mod imp {
        use std::cell::RefCell;

        use glib::{ParamSpec, ParamSpecObject, Value};
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
                static PROPERTIES: Lazy<Vec<ParamSpec>> =
                    Lazy::new(|| vec![ParamSpecObject::builder::<Provider>("provider").build()]);
                PROPERTIES.as_ref()
            }

            fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
                match pspec.name() {
                    "provider" => {
                        let provider = value.get().unwrap();
                        self.provider.replace(provider);
                    }
                    _ => unimplemented!(),
                }
            }

            fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
                match pspec.name() {
                    "provider" => self.provider.borrow().to_value(),
                    _ => unimplemented!(),
                }
            }

            fn constructed(&self) {
                self.parent_constructed();
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
                self.obj().set_child(Some(&hbox));
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
            glib::Object::new(&[])
        }
    }
}
