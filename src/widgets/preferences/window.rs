use super::camera_page::CameraPage;
use super::password_page::PasswordPage;
use crate::{
    backup::{
        Aegis, AndOTP, Backupable, Bitwarden, FreeOTP, Google, LegacyAuthenticator, Operation,
        Restorable, RestorableItem,
    },
    config,
    models::ProvidersModel,
    utils::spawn_tokio,
};
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action, spawn};
use once_cell::sync::OnceCell;
use tokio::time::{sleep, Duration};

mod imp {
    use super::*;
    use adw::subclass::{preferences_window::PreferencesWindowImpl, window::AdwWindowImpl};
    use glib::{
        subclass::{self, Signal},
        ParamSpec, ParamSpecBoolean, Value,
    };
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

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
        pub camera_page: CameraPage,
        pub password_page: PasswordPage,
        #[template_child]
        pub backup_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub restore_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child(id = "auto_lock_switch")]
        pub auto_lock: TemplateChild<gtk::Switch>,
        #[template_child(id = "dark_mode_switch")]
        pub dark_mode: TemplateChild<gtk::Switch>,
        #[template_child]
        pub dark_mode_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child(id = "lock_timeout_spin_btn")]
        pub lock_timeout: TemplateChild<gtk::SpinButton>,
        pub key_entries: RefCell<HashMap<String, gtk::PasswordEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "PreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesWindow;

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            let actions = gio::SimpleActionGroup::new();

            Self {
                settings,
                has_set_password: Cell::new(false), // Synced from the application
                camera_page: CameraPage::new(actions.clone()),
                password_page: PasswordPage::new(actions.clone()),
                actions,
                model: OnceCell::new(),
                backup_actions: gio::SimpleActionGroup::new(),
                restore_actions: gio::SimpleActionGroup::new(),
                auto_lock: TemplateChild::default(),
                dark_mode: TemplateChild::default(),
                lock_timeout: TemplateChild::default(),
                backup_group: TemplateChild::default(),
                restore_group: TemplateChild::default(),
                dark_mode_group: TemplateChild::default(),
                file_chooser: RefCell::default(),
                key_entries: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecBoolean::new(
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
                    Signal::builder("restore-completed", &[], <()>::static_type().into())
                        .action()
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "has-set-password" => {
                    let has_set_password = value.get().unwrap();
                    self.has_set_password.set(has_set_password);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "has-set-password" => self.has_set_password.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_actions();
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
        let window = glib::Object::new::<Self>(&[]).expect("Failed to create PreferencesWindow");
        window.imp().model.set(model).unwrap();
        window.setup_widgets();
        window
    }

    pub fn has_set_password(&self) -> bool {
        self.property("has-set-password")
    }

    pub fn set_has_set_password(&self, state: bool) {
        self.set_property("has-set-password", &state)
    }

    fn setup_widgets(&self) {
        let imp = self.imp();

        let style_manager = adw::StyleManager::default();
        imp.dark_mode_group
            .set_visible(!style_manager.system_supports_color_schemes());

        imp.settings
            .bind("dark-theme", &*imp.dark_mode, "active")
            .build();
        imp.settings
            .bind("auto-lock", &*imp.auto_lock, "active")
            .build();
        imp.settings
            .bind("auto-lock-timeout", &*imp.lock_timeout, "value")
            .build();

        imp.password_page
            .bind_property("has-set-password", self, "has-set-password")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.connect_local(
            "restore-completed",
            false,
            clone!(@weak self as win => @default-return None, move |_| {
                win.close();
                None
            }),
        );

        // FreeOTP is first in all of these lists, since its the way to backup Authenticator for use
        // with Authenticator. Others are sorted alphabetically.

        self.register_backup::<FreeOTP>(&["text/plain"]);
        self.register_backup::<Aegis>(&["application/json"]);
        self.register_backup::<AndOTP>(&["application/json"]);

        self.register_restore::<FreeOTP>(&["text/plain"]);
        self.register_restore::<Aegis>(&["application/json"]);
        self.register_restore::<AndOTP>(&["application/json"]);
        self.register_restore::<Bitwarden>(&["application/json"]);
        self.register_restore::<Google>(&[]);
        self.register_restore::<LegacyAuthenticator>(&["application/json"]);
    }

    fn register_backup<T: Backupable>(&self, filters: &'static [&str]) {
        let imp = self.imp();
        if T::ENCRYPTABLE {
            let row = adw::ExpanderRow::builder()
                .title(&T::title())
                .subtitle(&T::subtitle())
                .show_enable_switch(false)
                .enable_expansion(true)
                .use_underline(true)
                .build();
            let key_row = adw::ActionRow::builder()
                .title(&gettext("Key / Passphrase"))
                .subtitle(&gettext("The key that will be used to decrypt the vault"))
                .build();

            let key_entry = gtk::PasswordEntry::builder()
                .valign(gtk::Align::Center)
                .build();
            key_row.add_suffix(&key_entry);
            imp.key_entries
                .borrow_mut()
                .insert(format!("backup.{}", T::identifier()), key_entry);
            row.add_row(&key_row);

            let button_row = adw::ActionRow::new();
            let key_button = gtk::Button::builder()
                .valign(gtk::Align::Center)
                .halign(gtk::Align::End)
                .label(&gettext("Select File"))
                .action_name(&format!("backup.{}", T::identifier()))
                .build();
            button_row.add_suffix(&key_button);
            row.add_row(&button_row);

            imp.backup_group.add(&row);
        } else {
            let row = adw::ActionRow::builder()
                .title(&T::title())
                .subtitle(&T::subtitle())
                .activatable(true)
                .use_underline(true)
                .action_name(&format!("backup.{}", T::identifier()))
                .build();

            imp.backup_group.add(&row);
        }

        let model = imp.model.get().unwrap().clone();
        action!(
            imp.backup_actions,
            &T::identifier(),
            clone!(@weak self as win, @weak model => move |_, _| {
                let dialog = win.select_file(filters, Operation::Backup);
                dialog.connect_response(clone!(@weak model, @weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
                        let key = T::ENCRYPTABLE.then(|| {
                            win.encyption_key(Operation::Backup, &T::identifier())
                        }).flatten();
                        if let Err(err) = T::backup(&model, &d.file().unwrap(), key.as_deref()) {
                            tracing::warn!("Failed to create a backup {}", err);
                        }
                    }
                    d.destroy();
                }));
            })
        );
    }

    fn register_restore<T: Restorable>(&self, filters: &'static [&str]) {
        let imp = self.imp();
        if T::ENCRYPTABLE {
            let row = adw::ExpanderRow::builder()
                .title(&T::title())
                .subtitle(&T::subtitle())
                .show_enable_switch(false)
                .enable_expansion(true)
                .use_underline(true)
                .build();
            let key_row = adw::ActionRow::builder()
                .title(&gettext("Key / Passphrase"))
                .subtitle(&gettext("The key used to encrypt the vault"))
                .build();
            let key_entry = gtk::PasswordEntry::builder()
                .valign(gtk::Align::Center)
                .build();
            key_row.add_suffix(&key_entry);
            imp.key_entries
                .borrow_mut()
                .insert(format!("restore.{}", T::identifier()), key_entry);
            row.add_row(&key_row);

            let button_row = adw::ActionRow::new();
            let key_button = gtk::Button::builder()
                .valign(gtk::Align::Center)
                .halign(gtk::Align::End)
                .label(&gettext("Select File"))
                .action_name(&format!("restore.{}", T::identifier()))
                .build();
            button_row.add_suffix(&key_button);
            row.add_row(&button_row);
            imp.restore_group.add(&row);
        } else if T::SCANNABLE {
            let menu_button = gtk::MenuButton::builder()
                .css_classes(vec!["flat".to_string()])
                .halign(gtk::Align::Fill)
                .valign(gtk::Align::Center)
                .icon_name("qrscanner-symbolic")
                .tooltip_text(&gettext("Scan QR Code"))
                .menu_model(&{
                    let menu = gio::Menu::new();

                    menu.insert(
                        0,
                        Some(&gettext("_Camera")),
                        Some(&format!("restore.{}.camera", T::identifier())),
                    );

                    menu.insert(
                        1,
                        Some(&gettext("_Screenshot")),
                        Some(&format!("restore.{}.screenshot", T::identifier())),
                    );

                    menu
                })
                .build();

            let row = adw::ActionRow::builder()
                .title(&T::title())
                .subtitle(&T::subtitle())
                .activatable(true)
                .activatable_widget(&menu_button)
                .use_underline(true)
                .build();

            row.add_suffix(&menu_button);

            imp.restore_group.add(&row);
        } else {
            let row = adw::ActionRow::builder()
                .title(&T::title())
                .subtitle(&T::subtitle())
                .activatable(true)
                .use_underline(true)
                .action_name(&format!("restore.{}", T::identifier()))
                .build();

            imp.restore_group.add(&row);
        }
        if T::SCANNABLE {
            action!(
                imp.restore_actions,
                &format!("{}.camera", T::identifier()),
                clone!(@weak self as win, @weak imp.camera_page as camera_page => move |_, _| {
                    get_action!(win.imp().actions, @show_camera_page).activate(None);
                    spawn!(async move {
                        loop {
                            match camera_page.scan_from_camera().await {
                                Ok(code) => match T::restore_from_data(code.as_bytes(), None) {
                                    Ok(items) => {
                                        win.restore_items::<T, T::Item>(items);
                                        break;
                                    },
                                    Err(error) => {
                                        tracing::error!(concat!(
                                            "Encountered an error while trying to restore from a ",
                                            "scanned QR code: {}",
                                        ), error);
                                    },
                                },
                                Err(error) => {
                                    tracing::error!(concat!(
                                        "Encountered an error while trying to scan from the ",
                                        "camera: {}",
                                    ), error);
                                },
                            }

                            // Sleep for a second to avoid overloading the CPU if a code
                            // keeps scanning incorrectly.
                            spawn_tokio(async {
                                sleep(Duration::from_millis(1000)).await;
                            }).await;
                        }
                    });
                })
            );
            action!(
                imp.restore_actions,
                &format!("{}.screenshot", T::identifier()),
                clone!(@weak self as win, @weak imp.camera_page as camera_page => move |_, _| {
                    spawn!(async move {
                        match camera_page.scan_from_screenshot().await {
                            Ok(code) => match T::restore_from_data(code.as_bytes(), None) {
                                Ok(items) => {
                                    win.restore_items::<T, T::Item>(items);
                                },
                                Err(error) => {
                                    tracing::error!(concat!(
                                        "Encountered an error while trying to restore from a ",
                                        "scanned QR code: {}",
                                    ), error);

                                    win.add_toast(&adw::Toast::new(&gettext("Unable to restore accounts")));
                                },
                            },
                            Err(error) => {
                                tracing::error!("Encountered an error while trying to scan from the screenshot: {}", error);

                                win.add_toast(&adw::Toast::new(&gettext("Couldn't find a QR code")));
                            },
                        }
                    });
                })
            );
        } else {
            action!(
                imp.restore_actions,
                &T::identifier(),
                clone!(@weak self as win => move |_, _| {
                    let dialog = win.select_file(filters, Operation::Restore);
                    dialog.connect_response(clone!(@weak win => move |d, response| {
                        if response == gtk::ResponseType::Accept {
                            let key = T::ENCRYPTABLE.then(|| {
                                win.encyption_key(Operation::Restore, &T::identifier())
                            }).flatten();

                            match T::restore_from_file(&d.file().unwrap(), key.as_deref()) {
                                Ok(items) => {
                                    win.restore_items::<T, T::Item>(items);
                                },
                                Err(err) => {
                                    tracing::warn!("Failed to parse the selected file {}", err);
                                }
                            }
                        }
                        d.destroy();
                    }));
                })
            );
        }
    }

    fn encyption_key(&self, mode: Operation, identifier: &str) -> Option<glib::GString> {
        let identifier = match mode {
            Operation::Backup => format!("backup.{}", identifier),
            Operation::Restore => format!("restore.{}", identifier),
        };
        self.imp()
            .key_entries
            .borrow()
            .get(&identifier)
            .map(|entry| entry.text())
    }

    fn restore_items<T: Restorable<Item = Q>, Q: RestorableItem>(&self, items: Vec<Q>) {
        let model = self.imp().model.get().unwrap();
        items
            .iter()
            .map(move |item| item.restore(model))
            .for_each(|item| {
                if let Err(err) = item {
                    tracing::warn!("Failed to restore item {}", err);
                }
            });
        self.emit_by_name::<()>("restore-completed", &[]);
    }

    fn select_file(
        &self,
        filters: &'static [&str],
        operation: Operation,
    ) -> gtk::FileChooserNative {
        let native = match operation {
            Operation::Backup => gtk::FileChooserNative::new(
                Some(&gettext("Backup")),
                gtk::Window::NONE,
                gtk::FileChooserAction::Save,
                Some(&gettext("Select")),
                Some(&gettext("Cancel")),
            ),
            Operation::Restore => gtk::FileChooserNative::new(
                Some(&gettext("Restore")),
                gtk::Window::NONE,
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
        self.imp().file_chooser.replace(Some(native.clone()));
        native.show();
        native
    }

    fn setup_actions(&self) {
        let imp = self.imp();

        imp.camera_page
            .connect_map(clone!(@weak self as win => move |_| {
                win.set_search_enabled(false);
            }));

        imp.camera_page
            .connect_unmap(clone!(@weak self as win => move |_| {
                win.set_search_enabled(true);
            }));

        imp.password_page
            .connect_map(clone!(@weak self as win => move |_| {
                win.set_search_enabled(false);
            }));

        imp.password_page
            .connect_unmap(clone!(@weak self as win => move |_| {
                win.set_search_enabled(true);
            }));

        action!(
            imp.actions,
            "show_camera_page",
            clone!(@weak self as win, @weak imp.camera_page as camera_page => move |_, _| {
                win.present_subpage(&camera_page);
            })
        );
        action!(
            imp.actions,
            "show_password_page",
            clone!(@weak self as win, @weak imp.password_page as password_page => move |_, _| {
                win.present_subpage(&password_page);
            })
        );
        action!(
            imp.actions,
            "close_page",
            clone!(@weak self as win, @weak imp.camera_page as camera_page => move |_, _| {
                win.close_subpage();
                camera_page.imp().camera.stop();
            })
        );

        self.insert_action_group("preferences", Some(&imp.actions));
        self.insert_action_group("backup", Some(&imp.backup_actions));
        self.insert_action_group("restore", Some(&imp.restore_actions));
    }
}
