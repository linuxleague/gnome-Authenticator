use crate::{
    config,
    models::{Keyring, ProvidersModel, FAVICONS_PATH},
    widgets::{PreferencesWindow, ProvidersDialog, Window},
};
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use gtk_macros::{action, get_action};

mod imp {
    use super::*;
    use glib::{ParamSpec, WeakRef};
    use std::cell::{Cell, RefCell};

    // The basic struct that holds our state and widgets
    // (Ref)Cells are used for members which need to be mutable
    pub struct Application {
        pub window: RefCell<Option<WeakRef<Window>>>,
        pub model: ProvidersModel,
        pub locked: Cell<bool>,
        pub lock_timeout_id: RefCell<Option<glib::SourceId>>,
        pub can_be_locked: Cell<bool>,
        pub settings: gio::Settings,
    }

    // Sets up the basics for the GObject
    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "Application";
        type ParentType = gtk::Application;
        type Type = super::Application;

        // Initialize with default values
        fn new() -> Self {
            let model = ProvidersModel::new();
            let settings = gio::Settings::new(config::APP_ID);

            Self {
                window: RefCell::new(None),
                model,
                settings,
                can_be_locked: Cell::new(false),
                lock_timeout_id: RefCell::default(),
                locked: Cell::new(false),
            }
        }
    }

    // Overrides GObject vfuncs
    impl ObjectImpl for Application {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_boolean(
                        "locked",
                        "locked",
                        "locked",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_boolean(
                        "can-be-locked",
                        "can_be_locked",
                        "can be locked",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }
        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "locked" => {
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

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "locked" => self.locked.get().to_value(),
                "can-be-locked" => self.can_be_locked.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    // This is empty, but we still need to provide an
    // empty implementation for each type we subclass.
    impl GtkApplicationImpl for Application {}

    // Overrides GApplication vfuncs
    impl ApplicationImpl for Application {
        fn startup(&self, app: &Self::Type) {
            self.parent_startup(app);

            adw::functions::init();

            let app = app.downcast_ref::<super::Application>().unwrap();
            if let Some(ref display) = gtk::gdk::Display::default() {
                let p = gtk::CssProvider::new();
                gtk::CssProvider::load_from_resource(
                    &p,
                    "/com/belmoussaoui/Authenticator/style.css",
                );
                gtk::StyleContext::add_provider_for_display(display, &p, 500);
                let theme = gtk::IconTheme::for_display(display).unwrap();
                theme.add_resource_path("/com/belmoussaoui/Authenticator/icons/");
            }

            // - action! is a macro from gtk_macros
            //   that creates a GSimpleAction with a callback.
            //
            // - clone! is a macro from glib-rs that allows
            //   you to easily handle references in callbacks
            //   without refcycles or leaks.
            //   When you don't want the callback to keep the
            //   Object alive, pass as @weak. Otherwise, pass
            //   as @strong. Most of the time you will want
            //   to use @weak.
            action!(app, "quit", clone!(@weak app => move |_, _| app.quit()));

            action!(
                app,
                "preferences",
                clone!(@weak app, @weak self.model as model  => move |_,_| {
                    let active_window = app.active_window().unwrap();
                    let win = active_window.downcast_ref::<Window>().unwrap();

                    let preferences = PreferencesWindow::new(model);
                    preferences.set_has_set_password(app.can_be_locked());
                    preferences.connect_local("restore-completed", false, clone!(@weak win => @default-return None, move |_| {
                        win.providers().refilter();
                        None
                    })).unwrap();
                    preferences.connect_notify_local(Some("has-set-password"), clone!(@weak app => move |preferences, _| {
                        let state = preferences.has_set_password();
                        app.set_can_be_locked(state);
                    }));
                    preferences.set_transient_for(Some(&active_window));
                    preferences.show();
                })
            );

            // About
            action!(
                app,
                "about",
                clone!(@weak app => move |_, _| {
                    let window = app.active_window().unwrap();
                    let about_dialog = gtk::AboutDialogBuilder::new()
                        .program_name(&gettext("Authenticator"))
                        .modal(true)
                        .version(config::VERSION)
                        .comments(&gettext("Generate Two-Factor Codes"))
                        .website("https://gitlab.gnome.org/World/Authenticator")
                        .authors(vec!["Bilal Elmoussaoui".to_string(), "Maximiliano Sandoval".to_string(), "Christopher Davis".to_string()  ])
                        .artists(vec!["Alexandros Felekidis".to_string(), "Tobias Bernard".to_string()])
                        .translator_credits(&gettext("translator-credits"))
                        .logo_icon_name(config::APP_ID)
                        .license_type(gtk::License::Gpl30)
                        .transient_for(&window)
                        .build();

                    about_dialog.show();
                })
            );
            action!(
                app,
                "providers",
                clone!(@weak app,@weak self.model as model => move |_, _| {
                    let window = app.active_window().unwrap();
                    let providers = ProvidersDialog::new(model);
                    let win = window.downcast_ref::<Window>().unwrap();
                    providers.connect_local("changed", false, clone!(@weak win => @default-return None, move |_| {
                        win.providers().refilter();
                        None
                    })).unwrap();
                    providers.set_transient_for(Some(&window));
                    providers.show();
                })
            );

            action!(
                app,
                "lock",
                clone!(@weak app => move |_, _| {
                    app.set_locked(true);
                })
            );
            app.bind_property("can-be-locked", &get_action!(app, @lock), "enabled")
                .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
                .build();
            app.bind_property("locked", &get_action!(app, @preferences), "enabled")
                .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
                .build();
            app.bind_property("locked", &get_action!(app, @providers), "enabled")
                .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
                .build();

            app.connect_notify_local(Some("can-be-locked"), |app, _| {
                if !app.can_be_locked() {
                    app.cancel_lock_timeout();
                }
            });

            self.settings.connect_changed(
                None,
                clone!(@weak app => move |settings, key| {
                    match key {
                        "auto-lock" => {
                            match settings.boolean(key) {
                                true => app.restart_lock_timeout(),
                                false => app.cancel_lock_timeout(),
                            }
                        },
                        "auto-lock-timeout" => app.restart_lock_timeout(),
                        _ => ()
                    }
                }),
            );
        }

        fn activate(&self, app: &Self::Type) {
            if let Some(ref win) = *self.window.borrow() {
                let window = win.upgrade().unwrap();
                window.present();
                return;
            }

            let app = app.downcast_ref::<super::Application>().unwrap();
            let window = app.create_window();
            window.present();
            self.window.replace(Some(window.downgrade()));

            let has_set_password = Keyring::has_set_password().unwrap_or(false);

            app.set_resource_base_path(Some("/com/belmoussaoui/Authenticator"));
            app.set_accels_for_action("app.quit", &["<primary>q"]);
            app.set_accels_for_action("app.lock", &["<primary>l"]);
            app.set_accels_for_action("app.providers", &["<primary>p"]);
            app.set_accels_for_action("app.preferences", &["<primary>comma"]);
            app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
            app.set_accels_for_action("win.search", &["<primary>f"]);
            app.set_accels_for_action("win.add_account", &["<primary>n"]);

            app.set_locked(has_set_password);
            app.set_can_be_locked(has_set_password);

            // Start the timeout to lock the app if the auto-lock
            // setting is enabled.
            app.restart_lock_timeout();
        }
    }
}

// Creates a wrapper struct that inherits the functions
// from objects listed it @extends or interfaces it @implements.
// This is what allows us to do e.g. application.quit() on
// Application without casting.
glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, gio::ActionMap;
}

impl Application {
    pub fn run() {
        info!("Authenticator ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        std::fs::create_dir_all(&*FAVICONS_PATH.clone()).ok();
        Keyring::ensure_unlocked()
            .expect("Authenticator couldn't reach a secret service provider or unlock it");

        let app = glib::Object::new::<Application>(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &gio::ApplicationFlags::empty()),
        ])
        .unwrap();

        ApplicationExtManual::run(&app);
    }

    pub fn locked(&self) -> bool {
        self.property("locked").unwrap().get::<bool>().unwrap()
    }

    pub fn set_locked(&self, state: bool) {
        self.set_property("locked", &state)
            .expect("Failed to set locked property");
    }

    pub fn can_be_locked(&self) -> bool {
        self.property("can-be-locked")
            .unwrap()
            .get::<bool>()
            .unwrap()
    }

    pub fn set_can_be_locked(&self, state: bool) {
        self.set_property("can-be-locked", &state)
            .expect("Failed to set can-be-locked property");
    }

    fn create_window(&self) -> Window {
        let self_ = imp::Application::from_instance(self);
        Window::new(self_.model.clone(), &self.clone())
    }

    /// Starts or restarts the lock timeout.
    pub fn restart_lock_timeout(&self) {
        let self_ = imp::Application::from_instance(self);
        let auto_lock = self_.settings.boolean("auto-lock");
        let timeout = self_.settings.uint("auto-lock-timeout") * 60;

        if !auto_lock {
            return;
        }

        self.cancel_lock_timeout();

        if !self.locked() && self.can_be_locked() {
            let id = glib::timeout_add_seconds_local(
                timeout,
                clone!(@weak self as app => @default-return glib::Continue(false), move || {
                    app.set_locked(true);
                    glib::Continue(false)
                }),
            );
            self_.lock_timeout_id.replace(Some(id));
        }
    }

    pub fn cancel_lock_timeout(&self) {
        let self_ = imp::Application::from_instance(self);

        if let Some(id) = self_.lock_timeout_id.borrow_mut().take() {
            glib::source_remove(id);
        }
    }
}
