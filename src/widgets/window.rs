use crate::{
    application::Application,
    config,
    models::{Account, Keyring, ProvidersModel},
    widgets::{
        accounts::AccountDetailsPage,
        providers::{ProvidersList, ProvidersListView},
        AccountAddDialog, ErrorRevealer,
    },
    window_state,
};
use gettextrs::gettext;
use glib::{clone, signal::Inhibit};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
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
    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass;

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
        pub click_gesture: TemplateChild<gtk::GestureClick>,
        #[template_child]
        pub key_gesture: TemplateChild<gtk::EventControllerKey>,
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
    }

    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            Self {
                settings,
                providers: TemplateChild::default(),
                model: OnceCell::new(),
                account_details: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                click_gesture: TemplateChild::default(),
                key_gesture: TemplateChild::default(),
                deck: TemplateChild::default(),
                error_revealer: TemplateChild::default(),
                empty_status_page: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                accounts_stack: TemplateChild::default(),
                locked_img: TemplateChild::default(),
                title_stack: TemplateChild::default(),
                main_stack: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow, gio::ActionMap, gio::ActionGroup;
}

impl Window {
    pub fn new(model: ProvidersModel, app: &Application) -> Self {
        let window = glib::Object::new::<Window>(&[("application", app)]).unwrap();
        app.add_window(&window);

        if config::PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(model, app);
        window.setup_actions(app);
        window.set_view(View::Login); // Start by default in an empty state
        window.setup_signals(app);
        window
    }

    pub fn set_view(&self, view: View) {
        let self_ = imp::Window::from_instance(self);
        match view {
            View::Login => {
                self_.main_stack.set_visible_child_name("login");
                self_.search_entry.set_key_capture_widget(gtk::NONE_WIDGET);
                self_.password_entry.grab_focus();
            }
            View::Accounts => {
                self_.main_stack.set_visible_child_name("unlocked");
                self_.deck.set_visible_child_name("accounts");
                self_.deck.set_can_swipe_back(false);
                if self_.providers.model().get_n_items() == 0 {
                    if self_.model.get().unwrap().has_providers() {
                        // We do have at least one provider
                        // the 0 items comes from the search filter, so let's show an empty search
                        // page instead
                        self_.providers.set_view(ProvidersListView::NoSearchResults);
                    } else {
                        self_.accounts_stack.set_visible_child_name("empty");
                        self_.search_entry.set_key_capture_widget(gtk::NONE_WIDGET);
                    }
                } else {
                    self_.providers.set_view(ProvidersListView::List);
                    self_.accounts_stack.set_visible_child_name("accounts");
                    self_.search_entry.set_key_capture_widget(Some(self));
                }
            }
            View::Account(account) => {
                self_.main_stack.set_visible_child_name("unlocked");
                self_.deck.set_visible_child_name("account");
                self_.deck.set_can_swipe_back(true);
                self_.account_details.set_account(&account);
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

    fn init(&self, model: ProvidersModel, app: &Application) {
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

        self_.providers.model().connect_items_changed(
            clone!(@weak self as win, @weak app => move |model, _,_,_| {
            // We do a check on set_view to ensure we always use the right page
            if !app.locked() {
                win.set_view(View::Accounts);
            }
            }),
        );

        self.set_icon_name(Some(config::APP_ID));
        self_.empty_status_page.set_icon_name(Some(config::APP_ID));
        self_.locked_img.set_from_icon_name(Some(config::APP_ID));

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

        let search_entry = &*self_.search_entry;
        let search_btn = &*self_.search_btn;
        let providers = &*self_.providers;
        search_entry.connect_search_changed(clone!(@weak providers => move |entry| {
            let text = entry.get_text().to_string();
            providers.search(text);
        }));
        search_entry.connect_search_started(clone!(@weak search_btn => move |entry| {
            search_btn.set_active(true);
        }));
        search_entry.connect_stop_search(clone!(@weak search_btn => move |entry| {
            search_btn.set_active(false);
        }));

        let title_stack = &*self_.title_stack;
        search_btn.connect_toggled(clone!(@weak search_entry, @weak title_stack => move |btn| {
            if btn.get_active() {
                title_stack.set_visible_child_name("search");
                search_entry.grab_focus();
            } else {
                search_entry.set_text("");
                title_stack.set_visible_child_name("title");
            }
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
        let search_btn = &*self_.search_btn;
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

        let password_entry = &*self_.password_entry;
        action!(
            self,
            "unlock",
            clone!(@weak self as win, @weak password_entry, @weak app => move |_, _| {
                let password = password_entry.get_text();
                if Keyring::is_current_password(&password).unwrap_or_else(|err| {
                    debug!("Could not verify password: {:?}", err);
                    false
                }) {
                    password_entry.set_text("");
                    app.set_locked(false);
                    app.restart_lock_timeout();
                    win.set_view(View::Accounts);
                } else {
                    let win_ = imp::Window::from_instance(&win);
                    win_.error_revealer.popup(&gettext("Wrong Password"));
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
            .connect_activate(clone!(@weak self as win => move |_| {
                win.activate_action("unlock", None);
            }));

        // On each click or key pressed we restart the timeout.
        self_
            .click_gesture
            .connect_pressed(clone!(@weak app => move |_, _, _, _| {
                app.restart_lock_timeout();
            }));

        self_
            .key_gesture
            .connect_key_pressed(clone!(@weak app => move |_, _, _, _| {
                app.restart_lock_timeout();
                Inhibit(false)
            }));
    }
}
