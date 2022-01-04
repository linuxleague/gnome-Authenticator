use crate::{config, models::Keyring, widgets::ErrorRevealer};
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;
use std::cell::Cell;

mod imp {
    use super::*;
    use glib::{subclass, ParamSpec, ParamSpecBoolean, Value};
    use gtk::subclass::widget::WidgetImplExt;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences_password_page.ui")]
    pub struct PasswordPage {
        pub actions: OnceCell<gio::SimpleActionGroup>,
        pub has_set_password: Cell<bool>,
        #[template_child]
        pub current_password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub error_revealer: TemplateChild<ErrorRevealer>,
        #[template_child]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub confirm_password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub current_password_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        pub default_password_signal: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PasswordPage {
        const NAME: &'static str = "PasswordPage";
        type Type = super::PasswordPage;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                has_set_password: Cell::new(false), // state is synced later on from the window
                current_password_entry: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                confirm_password_entry: TemplateChild::default(),
                current_password_row: TemplateChild::default(),
                error_revealer: TemplateChild::default(),
                status_page: TemplateChild::default(),
                actions: OnceCell::new(),
                default_password_signal: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PasswordPage {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
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
    }
    impl WidgetImpl for PasswordPage {
        fn unmap(&self, widget: &Self::Type) {
            widget.reset();
            self.parent_unmap(widget);
        }
    }
    impl BoxImpl for PasswordPage {}
}

glib::wrapper! {
    pub struct PasswordPage(ObjectSubclass<imp::PasswordPage>) @extends gtk::Widget, gtk::Box;
}

impl PasswordPage {
    pub fn new(actions: gio::SimpleActionGroup) -> Self {
        let page = glib::Object::new::<Self>(&[]).expect("Failed to create PasswordPage");
        page.imp().actions.set(actions).unwrap();
        page.setup_widgets();
        page.setup_actions();
        page
    }

    pub fn has_set_password(&self) -> bool {
        self.property::<bool>("has-set-password")
    }

    pub fn set_has_set_password(&self, new_value: bool) {
        self.set_property("has-set-password", &new_value);
    }

    fn validate(&self) {
        let imp = self.imp();

        let current_password = imp.current_password_entry.text();
        let password = imp.password_entry.text();
        let password_repeat = imp.confirm_password_entry.text();

        let is_valid = if self.has_set_password() {
            password_repeat == password && current_password != password && password != ""
        } else {
            password_repeat == password && password != ""
        };

        get_action!(imp.actions.get().unwrap(), @save_password).set_enabled(is_valid);
    }

    fn setup_widgets(&self) {
        let imp = self.imp();

        imp.status_page.set_icon_name(Some(config::APP_ID));

        imp.password_entry
            .connect_changed(clone!(@weak self as page=> move |_| page.validate()));
        imp.confirm_password_entry
            .connect_changed(clone!(@weak self as page => move |_| page.validate()));

        self.reset_validation();
        // Reset the validation whenever the password state changes
        self.connect_notify_local(
            Some("has-set-password"),
            clone!(@weak self as page => move |_, _| {
                page.reset_validation();
            }),
        );
    }

    // Called when either the user sets/resets the password to bind/unbind the
    // the validation callback on the password entry
    fn reset_validation(&self) {
        let imp = self.imp();
        if self.has_set_password() {
            imp.current_password_entry
                .connect_changed(clone!(@weak self as page => move |_| page.validate()));
        } else if let Some(handler_id) = imp.default_password_signal.borrow_mut().take() {
            imp.current_password_entry.disconnect(handler_id);
        }
    }

    fn setup_actions(&self) {
        let imp = self.imp();

        let actions = imp.actions.get().unwrap();
        action!(
            actions,
            "save_password",
            clone!(@weak self as page => move |_, _| {
                page.save();
            })
        );

        action!(
            actions,
            "reset_password",
            clone!(@weak self as page => move |_,_| {
                page.reset_password();
            })
        );
        get_action!(actions, @save_password).set_enabled(false);

        self.bind_property(
            "has-set-password",
            &get_action!(actions, @reset_password),
            "enabled",
        )
        .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
        .build();
    }

    fn reset_password(&self) {
        let imp = self.imp();

        let current_password = imp.current_password_entry.text();

        if self.has_set_password()
            && !Keyring::is_current_password(&current_password).unwrap_or(false)
        {
            imp.error_revealer.popup(&gettext("Wrong Passphrase"));
            return;
        }
        if Keyring::reset_password().is_ok() {
            let imp = self.imp();
            let actions = imp.actions.get().unwrap();

            get_action!(actions, @close_page).activate(None);
            get_action!(actions, @save_password).set_enabled(false);
            self.set_has_set_password(false);
        }
    }

    pub fn reset(&self) {
        let imp = self.imp();

        imp.current_password_entry.get().set_text("");
        imp.password_entry.get().set_text("");
        imp.confirm_password_entry.get().set_text("");
    }

    fn save(&self) {
        let imp = self.imp();
        let actions = imp.actions.get().unwrap();

        let current_password = imp.current_password_entry.text();
        let password = imp.password_entry.text();

        if self.has_set_password()
            && !Keyring::is_current_password(&current_password).unwrap_or(false)
        {
            imp.error_revealer.popup(&gettext("Wrong Passphrase"));
            return;
        }

        if Keyring::set_password(&password).is_ok() {
            self.reset();
            get_action!(actions, @save_password).set_enabled(false);
            self.set_has_set_password(true);
            get_action!(actions, @close_page).activate(None);
        }
    }
}
