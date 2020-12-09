use super::password_page::PasswordPage;
use crate::backup::{
    AndOTP, Backupable, Bitwarden, FreeOTP, LegacyAuthenticator, Operation, Restorable,
};
use crate::config;
use crate::models::ProvidersModel;
use gettextrs::gettext;
use gio::prelude::*;
use gio::ActionMapExt;
use gio::{subclass::ObjectSubclass, SettingsExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::prelude::*;
use once_cell::sync::OnceCell;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::{
        preferences_window::PreferencesWindowImpl, window::WindowImpl as HdyWindowImpl,
    };
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    pub struct PreferencesWindow {
        pub settings: gio::Settings,
        pub model: OnceCell<ProvidersModel>,
        pub actions: gio::SimpleActionGroup,
        pub backup_actions: gio::SimpleActionGroup,
        pub restore_actions: gio::SimpleActionGroup,
        pub password_page: PasswordPage,
        #[template_child(id = "auto_lock_switch")]
        pub auto_lock: TemplateChild<gtk::Switch>,
        #[template_child(id = "dark_theme_switch")]
        pub dark_theme: TemplateChild<gtk::Switch>,
        #[template_child(id = "lock_timeout_spin_btn")]
        pub lock_timeout: TemplateChild<gtk::SpinButton>,
        #[template_child(id = "backup_group")]
        pub backup_group: TemplateChild<libhandy::PreferencesGroup>,
        #[template_child(id = "restore_group")]
        pub restore_group: TemplateChild<libhandy::PreferencesGroup>,
        pub file_chooser: RefCell<Option<gtk::FileChooserNative>>,
    }

    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "PreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = libhandy::PreferencesWindow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            let actions = gio::SimpleActionGroup::new();

            Self {
                settings,
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
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/preferences.ui");
            Self::bind_template_children(klass);
            klass.add_signal(
                "restore-completed",
                glib::SignalFlags::ACTION,
                &[],
                glib::Type::Unit,
            );
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_actions();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for PreferencesWindow {}
    impl WindowImpl for PreferencesWindow {}
    impl HdyWindowImpl for PreferencesWindow {}
    impl PreferencesWindowImpl for PreferencesWindow {}
}

glib_wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
        @extends gtk::Widget, gtk::Window, libhandy::Window, libhandy::PreferencesWindow;
}

impl PreferencesWindow {
    pub fn new(model: ProvidersModel) -> Self {
        let window = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create PreferencesWindow")
            .downcast::<PreferencesWindow>()
            .expect("Created object is of wrong type");
        let self_ = imp::PreferencesWindow::from_instance(&window);
        self_.model.set(model);
        window.setup_widgets();
        window
    }

    fn setup_widgets(&self) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        self_.settings.bind(
            "dark-theme",
            &self_.dark_theme.get(),
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );
        self_.settings.bind(
            "auto-lock",
            &self_.auto_lock.get(),
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );

        self_
            .auto_lock
            .get()
            .bind_property("active", &self_.lock_timeout.get(), "sensitive")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.register_backup::<FreeOTP>(&["text/plain"]);
        self.register_backup::<AndOTP>(&["application/json"]);

        self.register_restore::<FreeOTP>(&["text/plain"]);
        self.register_restore::<AndOTP>(&["application/json"]);
        self.register_restore::<Bitwarden>(&["application/json"]);
        self.register_restore::<LegacyAuthenticator>(&["application/json"]);
    }

    fn register_backup<T: Backupable>(&self, filters: &'static [&str]) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        let row = libhandy::ActionRowBuilder::new()
            .title(&T::title())
            .subtitle(&T::subtitle())
            .activatable(true)
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
                        T::backup(model, d.get_file().unwrap());
                    }
                    d.destroy();
                }));
            })
        );

        self_.backup_group.get().add(&row);
    }

    fn register_restore<T: Restorable>(&self, filters: &'static [&str]) {
        let self_ = imp::PreferencesWindow::from_instance(self);

        let row = libhandy::ActionRowBuilder::new()
            .title(&T::title())
            .subtitle(&T::subtitle())
            .activatable(true)
            .action_name(&format!("restore.{}", T::identifier()))
            .build();

        let model = self_.model.get().unwrap();
        action!(
            self_.restore_actions,
            &T::identifier(),
            clone!(@weak self as win, @weak model => move |_, _| {
                let dialog = win.select_file(filters, Operation::Restore);
                dialog.connect_response(clone!(@weak model, @weak win => move |d, response| {
                    if response == gtk::ResponseType::Accept {
                        T::restore(model, d.get_file().unwrap()).unwrap();
                        win.emit("restore-completed", &[]).unwrap();
                    }
                    d.destroy();
                }));
            })
        );

        self_.restore_group.get().add(&row);
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
