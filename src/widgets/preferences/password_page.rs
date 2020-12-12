use crate::config;
use crate::helpers::Keyring;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::{clone, glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;
use std::cell::Cell;

mod imp {

    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    #[derive(CompositeTemplate)]
    pub struct PasswordPage {
        pub actions: OnceCell<gio::SimpleActionGroup>,
        pub has_set_password: Cell<bool>,
        #[template_child]
        pub current_password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub confirm_password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub current_password_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub password_img: TemplateChild<gtk::Image>,
    }

    impl ObjectSubclass for PasswordPage {
        const NAME: &'static str = "PasswordPage";
        type Type = super::PasswordPage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let has_set_password = Keyring::has_set_password().unwrap_or(false);

            Self {
                has_set_password: Cell::new(has_set_password),
                current_password_entry: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                confirm_password_entry: TemplateChild::default(),
                current_password_row: TemplateChild::default(),
                password_img: TemplateChild::default(),
                actions: OnceCell::new(),
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
    pub fn new(actions: gio::SimpleActionGroup) -> Self {
        let page = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create PasswordPage")
            .downcast::<PasswordPage>()
            .expect("Created object is of wrong type");
        let self_ = imp::PasswordPage::from_instance(&page);
        self_.actions.set(actions).unwrap();
        page.setup_widgets();
        page.setup_actions();
        page
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

        get_action!(self_.actions.get().unwrap(), @save_password).set_enabled(is_valid);
    }

    fn setup_widgets(&self) {
        let self_ = imp::PasswordPage::from_instance(self);

        self_
            .password_img
            .get()
            .set_from_icon_name(Some(config::APP_ID));

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

        let actions = self_.actions.get().unwrap();
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
                page.reset();
            })
        );
        get_action!(actions, @save_password).set_enabled(false);
        get_action!(actions, @reset_password).set_enabled(self_.has_set_password.get());
    }

    fn reset(&self) {
        if Keyring::reset_password().is_ok() {
            let self_ = imp::PasswordPage::from_instance(self);
            let actions = self_.actions.get().unwrap();

            get_action!(actions, @close_page).activate(None);
            get_action!(actions, @save_password).set_enabled(false);
            get_action!(actions, @reset_password).set_enabled(false);
            self_.current_password_row.get().hide();
            self_.has_set_password.set(false);
        }
    }

    fn save(&self) {
        let self_ = imp::PasswordPage::from_instance(self);
        let actions = self_.actions.get().unwrap();

        let current_password = self_.current_password_entry.get().get_text().unwrap();
        let password = self_.password_entry.get().get_text().unwrap();

        if Keyring::has_set_password().unwrap_or(false)
            && !Keyring::is_current_password(&current_password).unwrap_or(false)
        {
            return;
        }

        if Keyring::set_password(&password).is_ok() {
            self_.current_password_row.get().show();
            self_.current_password_entry.get().set_text("");
            self_.password_entry.get().set_text("");
            self_.confirm_password_entry.get().set_text("");
            get_action!(actions, @save_password).set_enabled(false);
            get_action!(actions, @reset_password).set_enabled(true);
            get_action!(actions, @close_page).activate(None);
            self_.has_set_password.set(true);
        }
    }
}