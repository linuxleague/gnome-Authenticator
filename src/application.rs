use std::{collections::HashMap, str::FromStr};

use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, subclass::prelude::*};
use gtk_macros::{action, get_action};
use search_provider::{ResultID, ResultMeta, SearchProvider, SearchProviderImpl};

use crate::{
    config,
    models::{
        keyring, Account, OTPUri, Provider, ProvidersModel, FAVICONS_PATH, RUNTIME, SECRET_SERVICE,
    },
    utils::spawn_tokio_blocking,
    widgets::{PreferencesWindow, ProvidersDialog, Window},
};

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::subclass::prelude::*;
    use glib::{ParamSpec, ParamSpecBoolean, Value, WeakRef};
    use once_cell::sync::{Lazy, OnceCell};

    use super::*;

    // The basic struct that holds our state and widgets
    // (Ref)Cells are used for members which need to be mutable
    #[derive(Default)]
    pub struct Application {
        pub window: RefCell<Option<WeakRef<Window>>>,
        pub model: ProvidersModel,
        pub locked: Cell<bool>,
        pub lock_timeout_id: RefCell<Option<glib::SourceId>>,
        pub can_be_locked: Cell<bool>,
        pub settings: OnceCell<gio::Settings>,
        pub search_provider: RefCell<Option<SearchProvider<super::Application>>>,
    }

    // Sets up the basics for the GObject
    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type ParentType = adw::Application;
        type Type = super::Application;
    }

    // Overrides GObject vfuncs
    impl ObjectImpl for Application {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("is-locked").construct().build(),
                    ParamSpecBoolean::builder("can-be-locked")
                        .construct()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "is-locked" => {
                    let locked = value.get().unwrap();
                    self.locked.set(locked);
                }
                "can-be-locked" => {
                    let can_be_locked = value.get().unwrap();
                    self.can_be_locked.set(can_be_locked);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "is-locked" => self.locked.get().to_value(),
                "can-be-locked" => self.can_be_locked.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // Overrides GApplication vfuncs
    impl ApplicationImpl for Application {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            action!(app, "quit", clone!(@weak app => move |_, _| app.quit()));

            action!(
                app,
                "preferences",
                clone!(@weak app, @weak self.model as model  => move |_,_| {
                    let window = app.active_window();

                    let preferences = PreferencesWindow::new(model);
                    preferences.set_has_set_password(app.can_be_locked());
                    preferences.connect_restore_completed(clone!(@weak window =>move |_| {
                        window.providers().refilter();
                        window.imp().toast_overlay.add_toast(&adw::Toast::new(&gettext("Accounts restored successfully")));
                    }));
                    preferences.connect_has_set_password_notify(clone!(@weak app => move |_, state| {
                        app.set_can_be_locked(state);
                    }));
                    preferences.set_transient_for(Some(&window));
                    preferences.show();
                })
            );

            // About
            action!(
                app,
                "about",
                clone!(@weak app => move |_, _| {
                    let window = app.active_window();
                    gtk::AboutDialog::builder()
                        .program_name(&gettext("Authenticator"))
                        .modal(true)
                        .version(config::VERSION)
                        .comments(&gettext("Generate Two-Factor Codes"))
                        .website("https://gitlab.gnome.org/World/Authenticator")
                        .authors(vec!["Bilal Elmoussaoui".to_string(), "Maximiliano Sandoval".to_string(), "Christopher Davis".to_string(), "Julia Johannesen".to_string()  ])
                        .artists(vec!["Alexandros Felekidis".to_string(), "Tobias Bernard".to_string()])
                        .translator_credits(&gettext("translator-credits"))
                        .logo_icon_name(config::APP_ID)
                        .license_type(gtk::License::Gpl30)
                        .transient_for(&window)
                        .build()
                        .show();
                })
            );
            action!(
                app,
                "providers",
                clone!(@weak app,@weak self.model as model => move |_, _| {
                    let window = app.active_window();
                    let providers = ProvidersDialog::new(model);
                    providers.connect_changed(clone!(@weak window => move |_| {
                        window.providers().refilter();
                    }));
                    providers.set_transient_for(Some(&window));
                    providers.show();
                })
            );

            action!(
                app,
                "lock",
                clone!(@weak app => move |_, _| {
                    app.set_is_locked(true);
                })
            );
            app.bind_property("can-be-locked", &get_action!(app, @lock), "enabled")
                .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
                .build();
            app.bind_property("is-locked", &get_action!(app, @preferences), "enabled")
                .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
                .build();
            app.bind_property("is-locked", &get_action!(app, @providers), "enabled")
                .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
                .build();

            app.connect_can_be_locked_notify(|app, can_be_locked| {
                if !can_be_locked {
                    app.cancel_lock_timeout();
                }
            });

            self.settings.get().unwrap().connect_changed(
                None,
                clone!(@weak app => move |settings, key| {
                    match key {
                        "auto-lock" => {
                            if settings.boolean(key) {
                                app.restart_lock_timeout();
                            } else {
                                app.cancel_lock_timeout();
                            }
                        },
                        "auto-lock-timeout" => app.restart_lock_timeout(),
                        "dark-theme" => app.update_color_scheme(),
                        _ => ()
                    }
                }),
            );
            app.update_color_scheme();

            let search_provider_path = config::OBJECT_PATH;
            let search_provider_name = format!("{}.SearchProvider", config::APP_ID);

            let ctx = glib::MainContext::default();
            ctx.spawn_local(clone!(@strong app as application => async move {
                let imp = application.imp();
                match SearchProvider::new(application.clone(), search_provider_name, search_provider_path).await {
                    Ok(search_provider) => {
                        imp.search_provider.replace(Some(search_provider));
                    },
                    Err(err) => tracing::debug!("Could not start search provider: {}", err),
                };
            }));
        }

        fn activate(&self) {
            let app = self.obj();
            if let Some(ref win) = *self.window.borrow() {
                let window = win.upgrade().unwrap();
                window.present();
                return;
            }

            let window = Window::new(self.model.clone(), &app.clone());
            window.present();
            self.window.replace(Some(window.downgrade()));

            app.set_accels_for_action("app.quit", &["<primary>q"]);
            app.set_accels_for_action("app.lock", &["<primary>l"]);
            app.set_accels_for_action("app.providers", &["<primary>p"]);
            app.set_accels_for_action("app.preferences", &["<primary>comma"]);
            app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
            app.set_accels_for_action("win.search", &["<primary>f"]);
            app.set_accels_for_action("win.add_account", &["<primary>n"]);
            // Start the timeout to lock the app if the auto-lock
            // setting is enabled.
            app.restart_lock_timeout();
        }

        fn open(&self, files: &[gio::File], _hint: &str) {
            self.activate();
            let uris = files
                .iter()
                .filter_map(|f| OTPUri::from_str(&f.uri()).ok())
                .collect::<Vec<OTPUri>>();
            // We only handle a single URI (see the desktop file)
            if let Some(uri) = uris.get(0) {
                let window = self.obj().active_window();
                window.open_add_account(Some(uri))
            }
        }
    }
    // This is empty, but we still need to provide an
    // empty implementation for each type we subclass.
    impl GtkApplicationImpl for Application {}

    impl AdwApplicationImpl for Application {}
}

// Creates a wrapper struct that inherits the functions
// from objects listed it @extends or interfaces it @implements.
// This is what allows us to do e.g. application.quit() on
// Application without casting.
glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap;
}

impl Application {
    pub fn run() {
        tracing::info!("Authenticator ({})", config::APP_ID);
        tracing::info!("Version: {} ({})", config::VERSION, config::PROFILE);
        tracing::info!("Datadir: {}", config::PKGDATADIR);

        std::fs::create_dir_all(&*FAVICONS_PATH.clone()).ok();

        // To be removed in the upcoming release
        let settings = gio::Settings::new(config::APP_ID);
        if !settings.boolean("keyrings-migrated") {
            tracing::info!("Migrating the secrets to the file backend");
            let output: oo7::Result<()> = RUNTIME.block_on(async {
                oo7::migrate(
                    vec![
                        HashMap::from([("application", config::APP_ID), ("type", "token")]),
                        HashMap::from([("application", config::APP_ID), ("type", "password")]),
                    ],
                    false,
                )
                .await?;
                Ok(())
            });
            match output {
                Ok(_) => {
                    settings
                        .set_boolean("keyrings-migrated", true)
                        .expect("Failed to update settings");
                    tracing::info!("Secrets were migrated successfully");
                }
                Err(err) => {
                    tracing::error!("Failed to migrate your data {err}");
                }
            }
        }

        RUNTIME.block_on(async {
            let keyring = oo7::Keyring::new()
                .await
                .expect("Failed to start a location service");
            keyring
                .unlock()
                .await
                .expect("Failed to unlock the default collection");
            SECRET_SERVICE.set(keyring).unwrap()
        });

        let has_set_password =
            spawn_tokio_blocking(async { keyring::has_set_password().await.unwrap_or(false) });
        let app = glib::Object::new::<Application>(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &gio::ApplicationFlags::HANDLES_OPEN),
            ("resource-base-path", &"/com/belmoussaoui/Authenticator"),
            ("is-locked", &has_set_password),
            ("can-be-locked", &has_set_password),
        ]);
        // Only load the model if the app is not locked
        if !has_set_password {
            app.imp().model.load();
        }
        app.imp().settings.set(settings).unwrap();

        ApplicationExtManual::run(&app);
    }

    pub fn active_window(&self) -> Window {
        self.imp()
            .window
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
    }

    pub fn is_locked(&self) -> bool {
        self.property("is-locked")
    }

    pub fn set_is_locked(&self, state: bool) {
        self.set_property("is-locked", &state);
    }

    pub fn connect_is_locked_notify<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, bool) + 'static,
    {
        self.connect_notify_local(
            Some("is-locked"),
            clone!(@weak self as app => move |_, _| {
                let is_locked = app.is_locked();
                callback(&app, is_locked);
            }),
        )
    }

    pub fn can_be_locked(&self) -> bool {
        self.property("can-be-locked")
    }

    pub fn connect_can_be_locked_notify<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, bool) + 'static,
    {
        self.connect_notify_local(
            Some("can-be-locked"),
            clone!(@weak self as app => move |_, _| {
                let can_be_locked = app.can_be_locked();
                callback(&app, can_be_locked);
            }),
        )
    }

    pub fn set_can_be_locked(&self, state: bool) {
        self.set_property("can-be-locked", &state);
    }

    /// Starts or restarts the lock timeout.
    pub fn restart_lock_timeout(&self) {
        let imp = self.imp();
        let auto_lock = imp.settings.get().unwrap().boolean("auto-lock");
        let timeout = imp.settings.get().unwrap().uint("auto-lock-timeout") * 60;

        if !auto_lock {
            return;
        }

        self.cancel_lock_timeout();

        if !self.is_locked() && self.can_be_locked() {
            let id = glib::timeout_add_seconds_local(
                timeout,
                clone!(@weak self as app => @default-return glib::Continue(false), move || {
                    app.set_is_locked(true);
                    glib::Continue(false)
                }),
            );
            imp.lock_timeout_id.replace(Some(id));
        }
    }

    fn cancel_lock_timeout(&self) {
        if let Some(id) = self.imp().lock_timeout_id.borrow_mut().take() {
            id.remove();
        }
    }

    fn update_color_scheme(&self) {
        let manager = self.style_manager();
        if !manager.system_supports_color_schemes() {
            let color_scheme = if self.imp().settings.get().unwrap().boolean("dark-theme") {
                adw::ColorScheme::PreferDark
            } else {
                adw::ColorScheme::PreferLight
            };
            manager.set_color_scheme(color_scheme);
        }
    }

    fn account_provider_by_identifier(&self, id: &str) -> Option<(Provider, Account)> {
        let identifier = id.split(':').collect::<Vec<&str>>();
        let provider_id = identifier.get(0)?.parse::<u32>().ok()?;
        let account_id = identifier.get(1)?.parse::<u32>().ok()?;

        let provider = self.imp().model.find_by_id(provider_id)?;
        let account = provider.accounts_model().find_by_id(account_id)?;

        Some((provider, account))
    }
}

impl SearchProviderImpl for Application {
    fn launch_search(&self, terms: &[String], timestamp: u32) {
        self.activate();
        let window = self.active_window();
        window.imp().search_entry.set_text(&terms.join(" "));
        window.imp().search_btn.set_active(true);
        window.present_with_time(timestamp);
    }

    fn activate_result(&self, _identifier: ResultID, _terms: &[String], _timestamp: u32) {
        let notification = gio::Notification::new(&gettext("One-Time password copied"));
        notification.set_body(Some(&gettext("Password was copied successfully")));
        self.send_notification(None, &notification);
    }

    fn initial_result_set(&self, terms: &[String]) -> Vec<ResultID> {
        // don't show any results if the application is locked
        if self.is_locked() {
            vec![]
        } else {
            self.imp()
                .model
                .find_accounts(terms)
                .into_iter()
                .map(|account| format!("{}:{}", account.provider().id(), account.id()))
                .collect::<Vec<_>>()
        }
    }

    fn result_metas(&self, identifiers: &[ResultID]) -> Vec<ResultMeta> {
        identifiers
            .iter()
            .filter_map(|id| {
                self.account_provider_by_identifier(id)
                    .map(|(provider, account)| {
                        ResultMeta::builder(id.to_owned(), &account.name())
                            .description(&provider.name())
                            .clipboard_text(&account.otp().replace(' ', ""))
                            .build()
                    })
            })
            .collect::<Vec<_>>()
    }
}
