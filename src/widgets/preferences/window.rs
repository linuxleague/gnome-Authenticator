use super::password_page::PasswordPage;
use crate::{
    backup::{
        AndOTP, Backupable, FreeOTP, LegacyAuthenticator, Operation, Restorable, RestorableItem,
    },
    config,
    models::ProvidersModel,
};
use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::action;
use once_cell::sync::OnceCell;

mod imp {
    use super::*;
    use adw::subclass::{preferences_window::PreferencesWindowImpl, window::AdwWindowImpl};
    use glib::{
        subclass::{self, Signal},
        ParamSpec,
    };
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences.ui")]
    pub struct PreferencesWindow {
        pub settings: gio::Settings,
        pub model: OnceCell<ProvidersModel>,
        pub has_set_password: Cell<bool>,
        pub actions: gio::SimpleActionGroup,
        pub backup_actions: gio::SimpleActionGroup,
        pub restore_actions: gio::SimpleActionGroup,
        pub file_chooser: RefCell<Option<gtk::FileChooserNative>>,
        pub password_page: PasswordPage,
        #[template_child]
        pub backup_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub restore_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child(id = "auto_lock_switch")]
        pub auto_lock: TemplateChild<gtk::Switch>,
        #[template_child(id = "dark_theme_switch")]
        pub dark_theme: TemplateChild<gtk::Switch>,
        #[template_child(id = "lock_timeout_spin_btn")]
        pub lock_timeout: TemplateChild<gtk::SpinButton>,
    }

    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "PreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesWindow;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            let actions = gio::SimpleActionGroup::new();

            Self {
                settings,
                has_set_password: Cell::new(false), // Synced from the application
                password_page: PasswordPage::new(actions.clone()),
                actions,
                model: OnceCell::new(),
                backup_actions: gio::SimpleActionGroup::new(),
                restore_actions: gio::SimpleActionGroup::new(),
                auto_lock: TemplateChild::default(),
                dark_theme: TemplateChild::default(),
                lock_timeout: TemplateChild::default(),
                backup_group: TemplateChild::default(),
                restore_group: TemplateChild::default(),
                file_chooser: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::boolean(
                    "has-set-password",
                    "has set password",
                    "Has Set Password",
                    false,
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("restore-completed", &[], <()>::static_type())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.get_name() {
                "has-set-password" => {
                    let has_set_password = value.get().unwrap().unwrap();
                    self.has_set_password.set(has_set_password);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "has-set-password" => self.has_set_password.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_actions();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for PreferencesWindow {}
    impl WindowImpl for PreferencesWindow {}
    impl AdwWindowImpl for PreferencesWindow {}
    impl PreferencesWindowImpl for PreferencesWindow {}
}

glib::wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow;
}

impl PreferencesWindow {
    pub fn new(model: ProvidersModel) -> Self {
        let window = glib::Object::new(&[]).expect("Failed to create PreferencesWindow");
        let self_ = imp::PreferencesWindow::from_instance(&window);
        self_.model.set(model).unwrap();
        window.setup_widgets();
        window
    }

    pub fn has_set_password(&self) -> bool {
        self.get_property("has-set-password")
            .unwrap()
            .get::<bool>()
            .unwrap()
            .unwrap()
    }

    pub fn set_has_set_password(&self, state: bool) {
        self.set_property("has-set-password", &state).unwrap()
    }

    fn setup_widgets(&self) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        self_
            .settings
            .bind("dark-theme", &*self_.dark_theme, "active")
            .build();
        self_
            .settings
            .bind("auto-lock", &*self_.auto_lock, "active")
            .build();
        self_
            .settings
            .bind("auto-lock-timeout", &*self_.lock_timeout, "value")
            .build();

        self_
            .password_page
            .bind_property("has-set-password", self, "has-set-password")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.register_backup::<FreeOTP>(&["text/plain"]);
        self.register_backup::<AndOTP>(&["application/json"]);

        self.register_restore::<FreeOTP>(&["text/plain"]);
        self.register_restore::<AndOTP>(&["application/json"]);
        //self.register_restore::<Bitwarden>(&["application/json"]);
        self.register_restore::<LegacyAuthenticator>(&["application/json"]);
    }

    fn register_backup<T: Backupable>(&self, filters: &'static [&str]) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        let row = adw::ActionRowBuilder::new()
            .title(&T::title())
            .subtitle(&T::subtitle())
            .activatable(true)
            .use_underline(true)
            .action_name(&format!("backup.{}", T::identifier()))
            .build();

        let model = self_.model.get().unwrap().clone();
        action!(
            self_.backup_actions,
            &T::identifier(),
            clone!(@weak self as win, @weak model => move |_, _| {
                let dialog = win.select_file(filters, Operation::Backup);
                dialog.connect_response(clone!(@weak model, @weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
                        if let Err(err) = T::backup(&model, &d.get_file().unwrap()) {
                            warn!("Failed to create a backup {}", err);
                        }
                    }
                    d.destroy();
                }));
            })
        );

        self_.backup_group.add(&row);
    }

    fn register_restore<T: Restorable>(&self, filters: &'static [&str]) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        let row = adw::ActionRowBuilder::new()
            .title(&T::title())
            .subtitle(&T::subtitle())
            .activatable(true)
            .use_underline(true)
            .action_name(&format!("restore.{}", T::identifier()))
            .build();

        action!(
            self_.restore_actions,
            &T::identifier(),
            clone!(@weak self as win => move |_, _| {
                let dialog = win.select_file(filters, Operation::Restore);
                dialog.connect_response(clone!(@weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
                        match T::restore(&d.get_file().unwrap()) {
                            Ok(items) => {
                                win.restore_items::<T, T::Item>(items);
                            },
                            Err(err) => {
                                warn!("Failed to parse the selected file {}", err);
                            }
                        }
                    }
                    d.destroy();
                }));
            })
        );

        self_.restore_group.add(&row);
    }

    fn restore_items<T: Restorable<Item = Q>, Q: RestorableItem>(&self, items: Vec<Q>) {
        let self_ = imp::PreferencesWindow::from_instance(self);
        let model = self_.model.get().unwrap();
        items
            .iter()
            .map(move |item| T::restore_item(item, model))
            .filter(Result::is_ok)
            .for_each(|item| {
                if let Err(err) = item {
                    warn!("Failed to restore item {}", err);
                }
            });
        self.emit("restore-completed", &[]).unwrap();
    }

    fn select_file(
        &self,
        filters: &'static [&str],
        operation: Operation,
    ) -> gtk::FileChooserNative {
        let self_ = imp::PreferencesWindow::from_instance(self);

        let native = match operation {
            Operation::Backup => gtk::FileChooserNative::new(
                Some(&gettext("Backup")),
                gtk::NONE_WINDOW,
                gtk::FileChooserAction::Save,
                Some(&gettext("Select")),
                Some(&gettext("Cancel")),
            ),
            Operation::Restore => gtk::FileChooserNative::new(
                Some(&gettext("Restore")),
                gtk::NONE_WINDOW,
                gtk::FileChooserAction::Open,
                Some(&gettext("Select")),
                Some(&gettext("Cancel")),
            ),
        };

        native.set_modal(true);
        native.set_transient_for(Some(self));

        filters.iter().for_each(|f| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type(f);
            filter.set_name(Some(f));
            native.add_filter(&filter);
        });

        // Hold a reference to the file chooser
        self_.file_chooser.replace(Some(native.clone()));
        native.show();
        native
    }

    fn setup_actions(&self) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        action!(
            self_.actions,
            "show_password_page",
            clone!(@weak self as win, @weak self_.password_page as password_page => move |_, _| {
                win.present_subpage(&password_page);
            })
        );
        action!(
            self_.actions,
            "close_page",
            clone!(@weak self as win => move |_, _| {
                win.close_subpage();
            })
        );
        self.insert_action_group("preferences", Some(&self_.actions));
        self.insert_action_group("backup", Some(&self_.backup_actions));
        self.insert_action_group("restore", Some(&self_.restore_actions));
    }
}
