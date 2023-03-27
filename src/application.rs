use std::{collections::HashMap, str::FromStr};

use adw::prelude::*;
use futures_util::StreamExt;
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, clone},
    subclass::prelude::*,
};
use search_provider::ResultMeta;

use crate::{
    config,
    models::{
        keyring, start as start_search_provider, Account, OTPUri, Provider, ProvidersModel,
        SearchProviderAction, FAVICONS_PATH, RUNTIME, SECRET_SERVICE, SETTINGS,
    },
    utils::spawn_tokio_blocking,
    widgets::{PreferencesWindow, ProvidersDialog, Window},
};

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::subclass::prelude::*;

    use super::*;

    // The basic struct that holds our state and widgets
    // (Ref)Cells are used for members which need to be mutable
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Application)]
    pub struct Application {
        pub window: RefCell<Option<glib::WeakRef<Window>>>,
        pub model: ProvidersModel,
        #[property(get, set, construct)]
        pub is_locked: Cell<bool>,
        pub lock_timeout_id: RefCell<Option<glib::SourceId>>,
        #[property(get, set, construct)]
        pub can_be_locked: Cell<bool>,
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
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    // Overrides GApplication vfuncs
    impl ApplicationImpl for Application {
        fn startup(&self) {
            self.parent_startup();
            let app = self.obj();
            let quit_action = gio::ActionEntry::builder("quit")
                .activate(|app: &Self::Type, _, _| app.quit())
                .build();

            let preferences_action = gio::ActionEntry::builder("preferences")
                .activate(|app: &Self::Type, _, _| {
                    let model = &app.imp().model;
                    let window = app.active_window();
                    let preferences = PreferencesWindow::new(model);
                    preferences.set_has_set_password(app.can_be_locked());
                    preferences.connect_restore_completed(clone!(@weak window =>move |_| {
                        window.providers().refilter();
                        window.imp().toast_overlay.add_toast(adw::Toast::new(&gettext("Accounts restored successfully")));
                    }));
                    preferences.connect_has_set_password_notify(clone!(@weak app => move |pref| {
                        app.set_can_be_locked(pref.has_set_password());
                    }));
                    preferences.set_transient_for(Some(&window));
                    preferences.present();
                }).build();

            // About
            let about_action = gio::ActionEntry::builder("about")
                .activate(|app: &Self::Type, _, _| {
                    let window = app.active_window();
                    adw::AboutWindow::builder()
                        .application_name(gettext("Authenticator"))
                        .modal(true)
                        .version(config::VERSION)
                        .comments(gettext("Generate Two-Factor Codes"))
                        .website("https://gitlab.gnome.org/World/Authenticator")
                        .developers(vec![
                            "Bilal Elmoussaoui",
                            "Maximiliano Sandoval",
                            "Christopher Davis",
                            "Julia Johannesen",
                        ])
                        .artists(vec!["Alexandros Felekidis", "Tobias Bernard"])
                        .translator_credits(gettext("translator-credits"))
                        .application_icon(config::APP_ID)
                        .license_type(gtk::License::Gpl30)
                        .transient_for(&window)
                        .build()
                        .present();
                })
                .build();

            let providers_action = gio::ActionEntry::builder("providers")
                .activate(|app: &Self::Type, _, _| {
                    let model = &app.imp().model;
                    let window = app.active_window();
                    let providers = ProvidersDialog::new(model);
                    providers.connect_changed(clone!(@weak window => move |_| {
                        window.providers().refilter();
                    }));
                    providers.set_transient_for(Some(&window));
                    providers.present();
                })
                .build();

            let lock_action = gio::ActionEntry::builder("lock")
                .activate(|app: &Self::Type, _, _| app.set_is_locked(true))
                .build();

            app.add_action_entries([
                quit_action,
                about_action,
                lock_action,
                providers_action,
                preferences_action,
            ]);

            let lock_action = app.lookup_action("lock").unwrap();
            let preferences_action = app.lookup_action("preferences").unwrap();
            let providers_action = app.lookup_action("providers").unwrap();
            app.bind_property("can-be-locked", &lock_action, "enabled")
                .sync_create()
                .build();
            app.bind_property("is-locked", &preferences_action, "enabled")
                .invert_boolean()
                .sync_create()
                .build();
            app.bind_property("is-locked", &providers_action, "enabled")
                .invert_boolean()
                .sync_create()
                .build();

            app.connect_can_be_locked_notify(|app| {
                if !app.can_be_locked() {
                    app.cancel_lock_timeout();
                }
            });

            SETTINGS.connect_changed(
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

            let ctx = glib::MainContext::default();
            ctx.spawn_local(clone!(@strong app => async move {
                app.start_search_provider().await;
            }));
        }

        fn activate(&self) {
            let app = self.obj();
            if let Some(ref win) = *self.window.borrow() {
                let window = win.upgrade().unwrap();
                window.present();
                return;
            }

            let window = Window::new(&self.model, &app);
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
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Application {
    pub fn run() -> glib::ExitCode {
        tracing::info!("Authenticator ({})", config::APP_ID);
        tracing::info!("Version: {} ({})", config::VERSION, config::PROFILE);
        tracing::info!("Datadir: {}", config::PKGDATADIR);

        std::fs::create_dir_all(&*FAVICONS_PATH).ok();

        // To be removed in the upcoming release
        if !SETTINGS.boolean("keyrings-migrated") {
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
                    SETTINGS
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
                .expect("Failed to start the keyring service");
            keyring
                .unlock()
                .await
                .expect("Failed to unlock the default collection");
            SECRET_SERVICE.set(keyring).unwrap()
        });

        let has_set_password =
            spawn_tokio_blocking(async { keyring::has_set_password().await.unwrap_or(false) });
        let app = glib::Object::builder::<Application>()
            .property("application-id", config::APP_ID)
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .property("resource-base-path", "/com/belmoussaoui/Authenticator")
            .property("is-locked", has_set_password)
            .property("can-be-locked", has_set_password)
            .build();
        // Only load the model if the app is not locked
        if !has_set_password {
            app.imp().model.load();
        }

        app.run()
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

    /// Starts or restarts the lock timeout.
    pub fn restart_lock_timeout(&self) {
        let imp = self.imp();
        let auto_lock = SETTINGS.boolean("auto-lock");
        let timeout = SETTINGS.uint("auto-lock-timeout") * 60;

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
            let color_scheme = if SETTINGS.boolean("dark-theme") {
                adw::ColorScheme::PreferDark
            } else {
                adw::ColorScheme::PreferLight
            };
            manager.set_color_scheme(color_scheme);
        }
    }

    fn account_provider_by_identifier(&self, id: &str) -> Option<(Provider, Account)> {
        let identifier = id.split(':').collect::<Vec<&str>>();
        let provider_id = identifier.first()?.parse::<u32>().ok()?;
        let account_id = identifier.get(1)?.parse::<u32>().ok()?;

        let provider = self.imp().model.find_by_id(provider_id)?;
        let account = provider.accounts_model().find_by_id(account_id)?;

        Some((provider, account))
    }

    async fn start_search_provider(&self) {
        let mut receiver = match start_search_provider().await {
            Err(err) => {
                tracing::error!("Failed to start search provider {err}");
                return;
            }
            Ok(receiver) => receiver,
        };
        loop {
            let response = receiver.next().await.unwrap();
            match response {
                SearchProviderAction::LaunchSearch(terms, timestamp) => {
                    self.activate();
                    let window = self.active_window();
                    window.imp().search_entry.set_text(&terms.join(" "));
                    window.imp().search_btn.set_active(true);
                    window.present_with_time(timestamp);
                }
                SearchProviderAction::ActivateResult => {
                    let notification = gio::Notification::new(&gettext("One-Time password copied"));
                    notification.set_body(Some(&gettext("Password was copied successfully")));
                    self.send_notification(None, &notification);
                }
                SearchProviderAction::InitialResultSet(terms, sender) => {
                    // don't show any results if the application is locked
                    let response = if self.is_locked() {
                        vec![]
                    } else {
                        self.imp()
                            .model
                            .find_accounts(&terms)
                            .into_iter()
                            .map(|account| format!("{}:{}", account.provider().id(), account.id()))
                            .collect::<Vec<_>>()
                    };
                    let _ = sender.send(response).unwrap();
                }
                SearchProviderAction::ResultMetas(identifiers, sender) => {
                    let metas = identifiers
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
                        .collect::<Vec<_>>();
                    let _ = sender.send(metas).unwrap();
                }
            }
        }
    }
}
