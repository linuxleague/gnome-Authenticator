use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action, spawn};
use once_cell::sync::OnceCell;

use super::{camera_page::CameraPage, password_page::PasswordPage};
use crate::{
    backup::{
        Aegis, AndOTP, Backupable, Bitwarden, FreeOTP, Google, LegacyAuthenticator, Operation,
        Restorable, RestorableItem,
    },
    models::{ProvidersModel, SETTINGS},
};

mod imp {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    use adw::subclass::{preferences_window::PreferencesWindowImpl, window::AdwWindowImpl};
    use glib::{
        subclass::{self, Signal},
        ParamSpec, ParamSpecBoolean, Value,
    };
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences.ui")]
    pub struct PreferencesWindow {
        pub model: OnceCell<ProvidersModel>,
        pub has_set_password: Cell<bool>,
        pub actions: gio::SimpleActionGroup,
        pub backup_actions: gio::SimpleActionGroup,
        pub restore_actions: gio::SimpleActionGroup,
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
        #[template_child(id = "download_favicons_switch")]
        pub download_favicons: TemplateChild<gtk::Switch>,
        #[template_child(id = "download_favicons_metered_switch")]
        pub download_favicons_metered: TemplateChild<gtk::Switch>,
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
            let actions = gio::SimpleActionGroup::new();

            Self {
                has_set_password: Cell::default(), // Synced from the application
                camera_page: CameraPage::new(actions.clone()),
                password_page: PasswordPage::new(actions.clone()),
                actions,
                model: OnceCell::default(),
                backup_actions: gio::SimpleActionGroup::new(),
                restore_actions: gio::SimpleActionGroup::new(),
                auto_lock: TemplateChild::default(),
                dark_mode: TemplateChild::default(),
                download_favicons: TemplateChild::default(),
                download_favicons_metered: TemplateChild::default(),
                lock_timeout: TemplateChild::default(),
                backup_group: TemplateChild::default(),
                restore_group: TemplateChild::default(),
                dark_mode_group: TemplateChild::default(),
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
                vec![ParamSpecBoolean::builder("has-set-password")
                    .construct()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("restore-completed").action().build()]);
            SIGNALS.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "has-set-password" => {
                    let has_set_password = value.get().unwrap();
                    self.has_set_password.set(has_set_password);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "has-set-password" => self.has_set_password.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_actions();
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
        let window = glib::Object::new::<Self>(&[]);
        window.imp().model.set(model).unwrap();
        window.setup_widget();
        window
    }

    pub fn connect_restore_completed<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_local(
            "restore-completed",
            false,
            clone!(@weak self as win => @default-return None, move |_| {
                callback(&win);
                None
            }),
        )
    }

    pub fn has_set_password(&self) -> bool {
        self.property("has-set-password")
    }

    pub fn set_has_set_password(&self, state: bool) {
        self.set_property("has-set-password", &state)
    }

    pub fn connect_has_set_password_notify<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, bool) + 'static,
    {
        self.connect_notify_local(
            Some("has-set-password"),
            clone!(@weak self as win => move |_, _| {
                let has_set_password = win.has_set_password();
                callback(&win, has_set_password);
            }),
        )
    }

    fn setup_widget(&self) {
        let imp = self.imp();

        let style_manager = adw::StyleManager::default();
        imp.dark_mode_group
            .set_visible(!style_manager.system_supports_color_schemes());

        SETTINGS
            .bind("dark-theme", &*imp.dark_mode, "active")
            .build();
        SETTINGS
            .bind("download-favicons", &*imp.download_favicons, "active")
            .build();
        SETTINGS
            .bind(
                "download-favicons-metered",
                &*imp.download_favicons_metered,
                "active",
            )
            .build();
        SETTINGS
            .bind("auto-lock", &*imp.auto_lock, "active")
            .build();
        SETTINGS
            .bind("auto-lock-timeout", &*imp.lock_timeout, "value")
            .build();

        imp.password_page
            .bind_property("has-set-password", self, "has-set-password")
            .sync_create()
            .bidirectional()
            .build();

        // FreeOTP is first in all of these lists, since its the way to backup
        // Authenticator for use with Authenticator. Others are sorted
        // alphabetically.

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
                let ctx = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win, @weak model => async move {
                    if let Ok(file) = win.select_file(filters, Operation::Backup).await {
                        let key = T::ENCRYPTABLE.then(|| {
                            win.encyption_key(Operation::Backup, &T::identifier())
                        }).flatten();
                        if let Err(err) = T::backup(&model, &file, key.as_deref()) {
                            tracing::warn!("Failed to create a backup {}", err);
                        }
                    }
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
                        match camera_page.scan_from_camera().await {
                            Ok(code) => match T::restore_from_data(code.as_bytes(), None) {
                                Ok(items) => win.restore_items::<T, T::Item>(items),
                                Err(error) => {
                                    tracing::error!(concat!(
                                        "Encountered an error while trying to restore from a ",
                                        "scanned QR code: {}",
                                    ), error);

                                    get_action!(win.imp().actions, @close_page).activate(None);

                                    win.add_toast(&adw::Toast::new(&gettext("Unable to restore accounts")));
                                },
                            },
                            Err(error) => {
                                tracing::error!(
                                    "Encountered an error while trying to scan from the camera: {}",
                                    error,
                                );

                                get_action!(win.imp().actions, @close_page).activate(None);

                                win.add_toast(&adw::Toast::new(&gettext("Something went wrong")));
                            },
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
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(clone!(@weak win => async move {
                        if let Ok(file) = win.select_file(filters, Operation::Restore).await {
                            let key = T::ENCRYPTABLE.then(|| {
                                win.encyption_key(Operation::Restore, &T::identifier())
                            }).flatten();

                            match T::restore_from_file(&file, key.as_deref()) {
                                Ok(items) => {
                                    win.restore_items::<T, T::Item>(items);
                                },
                                Err(err) => {
                                    tracing::warn!("Failed to parse the selected file {}", err);
                                }
                            }
                        }
                    }));
                })
            );
        }
    }

    fn encyption_key(&self, mode: Operation, identifier: &str) -> Option<glib::GString> {
        let identifier = match mode {
            Operation::Backup => format!("backup.{identifier}",),
            Operation::Restore => format!("restore.{identifier}"),
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
        self.close();
    }

    async fn select_file(
        &self,
        filters: &'static [&str],
        operation: Operation,
    ) -> Result<gio::File, glib::Error> {
        let filters_model = gio::ListStore::new(gtk::FileFilter::static_type());
        filters.iter().for_each(|f| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type(f);
            filter.set_name(Some(f));
            filters_model.append(&filter);
        });

        match operation {
            Operation::Backup => {
                let dialog = gtk::FileDialog::builder()
                    .modal(true)
                    .filters(&filters_model)
                    .title(&gettext("Backup"))
                    .build();
                dialog.save_future(Some(self)).await
            }
            Operation::Restore => {
                let dialog = gtk::FileDialog::builder()
                    .modal(true)
                    .filters(&filters_model)
                    .title(&gettext("Restore"))
                    .build();
                dialog.open_future(Some(self)).await
            }
        }
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
