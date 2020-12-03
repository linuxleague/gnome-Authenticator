use super::password_page::PasswordPage;
use crate::config;
use gio::prelude::*;
use gio::ActionMapExt;
use gio::{subclass::ObjectSubclass, SettingsExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::PreferencesWindowExt;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::{
        preferences_window::PreferencesWindowImpl, window::WindowImpl as HdyWindowImpl,
    };

    #[derive(CompositeTemplate)]
    pub struct PreferencesWindow {
        pub settings: gio::Settings,
        pub actions: gio::SimpleActionGroup,
        pub password_page: PasswordPage,
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
        type ParentType = libhandy::PreferencesWindow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            let actions = gio::SimpleActionGroup::new();

            Self {
                settings,
                actions,
                password_page: PasswordPage::new(),
                auto_lock: TemplateChild::default(),
                dark_theme: TemplateChild::default(),
                lock_timeout: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/preferences.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
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
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create PreferencesWindow")
            .downcast::<PreferencesWindow>()
            .expect("Created object is of wrong type")
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
    }
}
