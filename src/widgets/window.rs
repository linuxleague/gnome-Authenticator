use gettextrs::gettext;
use glib::{clone, signal::Inhibit};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;

use crate::{
    application::Application,
    config,
    models::{keyring, Account, OTPUri, ProvidersModel},
    utils::spawn_tokio_blocking,
    widgets::{
        accounts::AccountDetailsPage,
        providers::{ProvidersList, ProvidersListView},
        AccountAddDialog, ErrorRevealer,
    },
};

#[derive(PartialEq, Eq, Debug)]
pub enum View {
    Login,
    Accounts,
    Account(Account),
}

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/window.ui")]
    pub struct Window {
        pub settings: gio::Settings,
        pub model: OnceCell<ProvidersModel>,
        #[template_child]
        pub main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub providers: TemplateChild<ProvidersList>,
        #[template_child]
        pub account_details: TemplateChild<AccountDetailsPage>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub deck: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub error_revealer: TemplateChild<ErrorRevealer>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub locked_img: TemplateChild<gtk::Image>,
        #[template_child]
        pub accounts_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub empty_status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub unlock_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            Self {
                settings,
                providers: TemplateChild::default(),
                model: OnceCell::default(),
                account_details: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                deck: TemplateChild::default(),
                error_revealer: TemplateChild::default(),
                empty_status_page: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                accounts_stack: TemplateChild::default(),
                locked_img: TemplateChild::default(),
                title_stack: TemplateChild::default(),
                main_stack: TemplateChild::default(),
                unlock_button: TemplateChild::default(),
                toast_overlay: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("win.search", None, move |win, _, _| {
                let search_btn = &win.imp().search_btn;
                search_btn.set_active(!search_btn.is_active());
            });

            klass.install_action("win.back", None, move |win, _, _| {
                // Always return back to accounts list
                win.set_view(View::Accounts);
            });

            klass.install_action("win.unlock", None, move |win, _, _| {
                let imp = win.imp();
                let app = win.app();
                let password = imp.password_entry.text();
                let is_current_password = spawn_tokio_blocking(async move {
                    keyring::is_current_password(&password)
                        .await
                        .unwrap_or_else(|err| {
                            tracing::debug!("Could not verify password: {:?}", err);
                            false
                        })
                });
                if is_current_password {
                    imp.password_entry.set_text("");
                    app.set_is_locked(false);
                    app.restart_lock_timeout();
                    win.set_view(View::Accounts);
                    imp.model.get().unwrap().load();
                } else {
                    imp.error_revealer.popup(&gettext("Wrong Password"));
                }
            });
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {
        fn enable_debugging(&self, toggle: bool) -> bool {
            if config::PROFILE != "Devel" {
                tracing::warn!("Inspector is disabled for non development builds");
                false
            } else {
                self.parent_enable_debugging(toggle)
            }
        }

        fn close_request(&self) -> Inhibit {
            self.parent_close_request();
            if let Err(err) = self.obj().save_window_state() {
                tracing::warn!("Failed to save window state {:#?}", err);
            }
            Inhibit(false)
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Native, gtk::Root;
}

#[gtk::template_callbacks]
impl Window {
    pub fn new(model: ProvidersModel, app: &Application) -> Self {
        let window = glib::Object::new::<Window>(&[("application", app)]);
        app.add_window(&window);

        if config::PROFILE == "Devel" {
            window.style_context().add_class("devel");
        }
        window.init(model, app);
        window.setup_actions(app);
        window.set_view(View::Accounts); // Start by default in the accounts state
        window.setup_signals(app);
        window
    }

    pub fn set_view(&self, view: View) {
        let imp = self.imp();
        match view {
            View::Login => {
                self.set_default_widget(Some(&*imp.unlock_button));
                imp.main_stack.set_visible_child_name("login");
                imp.search_entry.set_key_capture_widget(gtk::Widget::NONE);
                imp.password_entry.grab_focus();
            }
            View::Accounts => {
                self.set_default_widget(gtk::Widget::NONE);
                imp.main_stack.set_visible_child_name("unlocked");
                imp.deck.set_visible_child_name("accounts");
                imp.deck.set_can_navigate_back(false);
                if imp.providers.model().n_items() == 0 {
                    if imp.model.get().unwrap().has_providers() {
                        // We do have at least one provider
                        // the 0 items comes from the search filter, so let's show an empty search
                        // page instead
                        imp.providers.set_view(ProvidersListView::NoSearchResults);
                    } else {
                        imp.accounts_stack.set_visible_child_name("empty");
                        imp.search_entry.set_key_capture_widget(gtk::Widget::NONE);
                    }
                } else {
                    imp.providers.set_view(ProvidersListView::List);
                    imp.accounts_stack.set_visible_child_name("accounts");
                    imp.search_entry.set_key_capture_widget(Some(self));
                }
            }
            View::Account(account) => {
                self.set_default_widget(gtk::Widget::NONE);
                imp.search_entry.set_key_capture_widget(gtk::Widget::NONE);
                imp.main_stack.set_visible_child_name("unlocked");
                imp.deck.set_visible_child_name("account");
                imp.deck.set_can_navigate_back(true);
                imp.account_details.set_account(&account);
            }
        }
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(&toast);
    }

    pub fn open_add_account(&self, otp_uri: Option<&OTPUri>) {
        let imp = self.imp();

        let model = imp.model.get().unwrap();

        let dialog = AccountAddDialog::new(model.clone());
        dialog.set_transient_for(Some(self));
        if let Some(uri) = otp_uri {
            dialog.set_from_otp_uri(uri);
        }

        dialog.connect_added(clone!(@weak self as win => move |_| {
            win.providers().refilter();
        }));
        dialog.show();
    }

    pub fn providers(&self) -> ProvidersList {
        self.imp().providers.clone()
    }

    fn init(&self, model: ProvidersModel, app: &Application) {
        let imp = self.imp();
        imp.model.set(model.clone()).unwrap();
        imp.providers.set_model(model.clone());

        imp.providers.model().connect_items_changed(
            clone!(@weak self as win, @weak app => move |_, _,_,_| {
            // We do a check on set_view to ensure we always use the right page
            if !app.is_locked() {
                win.set_view(View::Accounts);
            }
            }),
        );

        self.set_icon_name(Some(config::APP_ID));
        imp.empty_status_page.set_icon_name(Some(config::APP_ID));
        imp.locked_img.set_from_icon_name(Some(config::APP_ID));

        // load latest window state
        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");

        if width > -1 && height > -1 {
            self.set_default_size(width, height);
        }

        let is_maximized = imp.settings.boolean("is-maximized");
        if is_maximized {
            self.maximize();
        }
        imp.account_details.set_providers_model(model);
    }

    fn setup_actions(&self, app: &Application) {
        action!(
            self,
            "add_account",
            clone!(@weak self as win => move |_,_| {
                win.open_add_account(None);
            })
        );
        app.bind_property("is-locked", &get_action!(self, @add_account), "enabled")
            .invert_boolean()
            .sync_create()
            .build();
    }

    fn setup_signals(&self, app: &Application) {
        app.connect_is_locked_notify(clone!(@weak self as win => move |_, is_locked| {
            if is_locked{
                win.set_view(View::Login);
            } else {
                win.set_view(View::Accounts);
            };
        }));
        if app.is_locked() {
            self.set_view(View::Login);
        }
    }

    fn app(&self) -> Application {
        self.application()
            .unwrap()
            .downcast::<Application>()
            .unwrap()
    }

    fn save_window_state(&self) -> anyhow::Result<()> {
        let settings = &self.imp().settings;
        let size = self.default_size();
        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;

        settings.set_boolean("is-maximized", self.is_maximized())?;
        Ok(())
    }

    #[template_callback]
    fn on_password_entry_activate(&self) {
        WidgetExt::activate_action(self, "win.unlock", None).unwrap();
    }

    #[template_callback]
    fn on_account_removed(&self, account: Account) {
        let provider = account.provider();
        account.delete().unwrap();
        provider.remove_account(&account);
        self.providers().refilter();
        self.set_view(View::Accounts);
    }

    #[template_callback]
    fn on_provider_changed(&self) {
        self.providers().refilter();
    }

    #[template_callback]
    fn on_account_shared(&self, account: Account) {
        self.set_view(View::Account(account));
    }

    #[template_callback]
    fn on_gesture_click_pressed(&self) {
        self.app().restart_lock_timeout();
    }

    #[template_callback]
    fn on_key_pressed(&self) -> Inhibit {
        self.app().restart_lock_timeout();
        Inhibit(false)
    }

    #[template_callback]
    fn on_search_changed(&self, entry: &gtk::SearchEntry) {
        let text = entry.text().to_string();
        self.imp().providers.search(text);
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
}
