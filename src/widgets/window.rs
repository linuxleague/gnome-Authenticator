use crate::{
    application::Application,
    config,
    helpers::Keyring,
    models::{Account, ProvidersModel},
    widgets::{accounts::QRCodePage, providers::ProvidersList, AccountAddDialog},
    window_state,
};
use gio::subclass::ObjectSubclass;
use glib::{clone, signal::Inhibit, subclass::prelude::*};
use gtk::{gio, glib, prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;

#[derive(PartialEq, Debug)]
pub enum View {
    Login,
    Accounts,
    Account(Account),
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::application_window::ApplicationWindowImpl as HdyApplicationWindowImpl;

    #[derive(Debug, CompositeTemplate)]
    pub struct Window {
        pub settings: gio::Settings,
        pub providers: ProvidersList,
        pub qrcode_page: QRCodePage,
        pub model: OnceCell<ProvidersModel>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub deck: TemplateChild<libhandy::Leaflet>,
        #[template_child]
        pub container: TemplateChild<gtk::Box>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub locked_img: TemplateChild<gtk::Image>,
    }

    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = libhandy::ApplicationWindow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            Self {
                settings,
                providers: ProvidersList::new(),
                model: OnceCell::new(),
                qrcode_page: QRCodePage::new(),
                search_entry: TemplateChild::default(),
                deck: TemplateChild::default(),
                container: TemplateChild::default(),
                search_bar: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                locked_img: TemplateChild::default(),
            }
        }
        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/window.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();

            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl HdyApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, libhandy::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl Window {
    pub fn new(model: ProvidersModel, app: &Application) -> Self {
        let window = glib::Object::new::<Window>(&[("application", app)]).unwrap();
        app.add_window(&window);

        if config::PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(model);
        window.setup_actions(app);
        window.set_view(View::Login); // Start by default in an empty state
        window.setup_signals(app);
        window
    }

    pub fn set_view(&self, view: View) {
        let self_ = imp::Window::from_instance(self);
        match view {
            View::Login => {
                self_.deck.get().set_visible_child_name("login");
                self_
                    .search_entry
                    .get()
                    .set_key_capture_widget(gtk::NONE_WIDGET);
                self_.password_entry.get().grab_focus();
            }
            View::Accounts => {
                self_.deck.get().set_visible_child_name("accounts");

                //self_.search_entry.get().set_key_capture_widget(Some(self));
            }
            View::Account(account) => {
                self_.deck.get().set_visible_child_name("account");
                self_.qrcode_page.set_account(&account);
            }
        }
    }

    fn open_add_account(&self) {
        let self_ = imp::Window::from_instance(self);

        let model = self_.model.get().unwrap();

        let dialog = AccountAddDialog::new(model.clone());
        dialog.set_transient_for(Some(self));

        dialog
            .connect_local(
                "added",
                false,
                clone!(@weak self as win => move |_| {
                    win.providers().refilter();
                    None
                }),
            )
            .unwrap();

        dialog.show();
    }

    pub fn providers(&self) -> ProvidersList {
        let self_ = imp::Window::from_instance(self);
        self_.providers.clone()
    }

    fn init(&self, model: ProvidersModel) {
        let self_ = imp::Window::from_instance(self);
        self_.model.set(model.clone()).unwrap();
        self_.providers.set_model(model);
        self_
            .providers
            .connect_local(
                "shared",
                false,
                clone!(@weak self as win => move |args| {
                        let account = args.get(1).unwrap().get::<Account>().unwrap().unwrap();
                    win.set_view(View::Account(account));
                    None
                }),
            )
            .unwrap();

        self.set_icon_name(Some(config::APP_ID));
        self_
            .locked_img
            .get()
            .set_from_icon_name(Some(config::APP_ID));

        // load latest window state
        window_state::load(&self, &self_.settings);
        // save window state on delete event
        self.connect_close_request(clone!(@weak self_.settings as settings => move |window| {
            if let Err(err) = window_state::save(&window, &settings) {
                warn!("Failed to save window state {:#?}", err);
            }
            Inhibit(false)
        }));

        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        gtk_macros::get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.set_help_overlay(Some(&shortcuts));

        self_.container.get().append(&self_.providers);

        let page = self_.deck.get().append(&self_.qrcode_page).unwrap();
        page.set_name("account");

        self_
            .search_bar
            .get()
            .bind_property("search-mode-enabled", &self_.search_btn.get(), "active")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        let search_btn = self_.search_btn.get();
        self_.search_entry.get().connect_search_changed(
            clone!(@weak self_.providers as providers => move |entry| {
                let text = entry.get_text().unwrap().to_string();
                providers.search(text);
            }),
        );
        self_
            .search_entry
            .get()
            .connect_search_started(clone!(@weak search_btn => move |entry| {
                search_btn.set_active(true);
            }));
        self_
            .search_entry
            .get()
            .connect_stop_search(clone!(@weak search_btn => move |entry| {
                entry.set_text("");
                search_btn.set_active(false);
            }));

        let gtk_settings = gtk::Settings::get_default().unwrap();
        self_
            .settings
            .bind(
                "dark-theme",
                &gtk_settings,
                "gtk-application-prefer-dark-theme",
            )
            .build();
    }

    fn setup_actions(&self, app: &Application) {
        let self_ = imp::Window::from_instance(self);
        let search_btn = self_.search_btn.get();
        action!(
            self,
            "search",
            clone!(@weak search_btn => move |_,_| {
                search_btn.set_active(!search_btn.get_active());
            })
        );

        action!(
            self,
            "back",
            clone!(@weak self as win => move |_, _| {
                // Always return back to accounts list
                win.set_view(View::Accounts);
            })
        );

        action!(
            self,
            "add_account",
            clone!(@weak self as win => move |_,_| {
                win.open_add_account();
            })
        );
        app.bind_property("locked", &get_action!(self, @add_account), "enabled")
            .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
            .build();

        let password_entry = self_.password_entry.get();
        action!(
            self,
            "unlock",
            clone!(@weak self as win, @weak password_entry, @weak app => move |_, _| {
                let password = password_entry.get_text().unwrap();
                if Keyring::is_current_password(&password).unwrap() {
                    password_entry.set_text("");
                    app.set_locked(false);
                    win.set_view(View::Accounts);
                }
            })
        );
    }

    fn setup_signals(&self, app: &Application) {
        app.connect_local(
            "notify::locked",
            false,
            clone!(@weak app, @weak self as win => move |_| {
                if app.locked(){
                    win.set_view(View::Login);
                } else {
                    win.set_view(View::Accounts);
                };
                None
            }),
        )
        .unwrap();

        let self_ = imp::Window::from_instance(self);

        self_
            .password_entry
            .get()
            .connect_activate(clone!(@weak self as win => move |_| {
                win.activate_action("unlock", None);
            }));
    }
}
