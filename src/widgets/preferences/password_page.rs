use crate::helpers::Keyring;
use std::cell::Cell;

use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};

mod imp {

    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    #[derive(CompositeTemplate)]
    pub struct PasswordPage {
        // actions: gio::SimpleActionGroup,
        pub has_set_password: Cell<bool>,

        #[template_child(id = "current_password_entry")]
        pub current_password_entry: TemplateChild<gtk::PasswordEntry>,

        #[template_child(id = "password_entry")]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,

        #[template_child(id = "confirm_password_entry")]
        pub confirm_password_entry: TemplateChild<gtk::PasswordEntry>,

        #[template_child(id = "current_password_row")]
        pub current_password_row: TemplateChild<libhandy::ActionRow>,
    }

    impl ObjectSubclass for PasswordPage {
        const NAME: &'static str = "PasswordPage";
        type Type = super::PasswordPage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let has_set_password = Keyring::has_set_password().unwrap_or_else(|_| false);

            Self {
                has_set_password: Cell::new(has_set_password),
                current_password_entry: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                confirm_password_entry: TemplateChild::default(),
                current_password_row: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource(
                "/com/belmoussaoui/Authenticator/preferences_password_page.ui",
            );
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for PasswordPage {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
            obj.setup_actions();
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for PasswordPage {}
    impl BoxImpl for PasswordPage {}
}

glib_wrapper! {
    pub struct PasswordPage(ObjectSubclass<imp::PasswordPage>) @extends gtk::Widget, gtk::Box;
}

impl PasswordPage {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create PasswordPage")
            .downcast::<PasswordPage>()
            .expect("Created object is of wrong type")
    }

    fn validate(&self) {
        let self_ = imp::PasswordPage::from_instance(self);

        let current_password = self_.current_password_entry.get().get_text().unwrap();
        let password = self_.password_entry.get().get_text().unwrap();
        let password_repeat = self_.confirm_password_entry.get().get_text().unwrap();

        let is_valid = if self_.has_set_password.get() {
            password_repeat == password && current_password != password && password != ""
        } else {
            password_repeat == password && password != ""
        };

        // get_action!(self.actions, @save_password).set_enabled(is_valid);
    }

    fn setup_widgets(&self) {
        let self_ = imp::PasswordPage::from_instance(self);

        self_
            .password_entry
            .get()
            .connect_changed(clone!(@weak self as page=> move |_| page.validate()));
        self_
            .confirm_password_entry
            .get()
            .connect_changed(clone!(@weak self as page => move |_| page.validate()));

        if !self_.has_set_password.get() {
            self_.current_password_row.get().hide();
        } else {
            self_
                .current_password_entry
                .get()
                .connect_changed(clone!(@weak self as page => move |_| page.validate()));
        }
    }

    fn setup_actions(&self) {
        let self_ = imp::PasswordPage::from_instance(self);

        /*action!(
            self.actions,
            "save_password",
            clone!(@strong page => move |_, _| {
                page.save();
            })
        );

        action!(
            self.actions,
            "reset_password",
            clone!(@strong page => move |_,_| {
                page.reset();
            })
        );
        get_action!(self.actions, @save_password).set_enabled(false);
        get_action!(self.actions, @reset_password).set_enabled(self.has_set_password.get());*/
    }

    fn reset(&self) {
        if Keyring::reset_password().is_ok() {
            let self_ = imp::PasswordPage::from_instance(self);

            // get_action!(self.actions, @close_page).activate(None);
            // get_action!(self.actions, @save_password).set_enabled(false);
            // get_action!(self.actions, @reset_password).set_enabled(false);
            self_.current_password_row.get().hide();
            self_.has_set_password.set(false);
        }
    }

    fn save(&self) {
        let self_ = imp::PasswordPage::from_instance(self);

        let current_password = self_.current_password_entry.get().get_text().unwrap();
        let password = self_.password_entry.get().get_text().unwrap();

        if Keyring::has_set_password().unwrap_or(false) {
            if !Keyring::is_current_password(&current_password).unwrap_or(false) {
                return;
            }
        }

        if Keyring::set_password(&password).is_ok() {
            self_.current_password_row.get().show();
            self_.current_password_entry.get().set_text("");
            self_.password_entry.get().set_text("");
            self_.confirm_password_entry.get().set_text("");
            // get_action!(self.actions, @save_password).set_enabled(false);
            // get_action!(self.actions, @reset_password).set_enabled(true);
            // get_action!(self.actions, @close_page).activate(None);
            self_.has_set_password.set(true);
        }
    }
}
