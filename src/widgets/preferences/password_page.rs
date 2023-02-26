use std::cell::{Cell, RefCell};

use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, clone, subclass::InitializingObject},
    prelude::*,
    subclass::prelude::*,
};
use gtk_macros::{action, get_action};

use crate::{config, models::keyring, utils::spawn_tokio, widgets::ErrorRevealer};

mod imp {
    use once_cell::sync::OnceCell;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::PasswordPage)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences_password_page.ui")]
    pub struct PasswordPage {
        pub actions: OnceCell<gio::SimpleActionGroup>,
        #[property(get, set, construct)]
        pub has_set_password: Cell<bool>,
        #[template_child]
        pub current_password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub error_revealer: TemplateChild<ErrorRevealer>,
        #[template_child]
        pub password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub confirm_password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        pub default_password_signal: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PasswordPage {
        const NAME: &'static str = "PasswordPage";
        type Type = super::PasswordPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PasswordPage {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            let page = self.obj();
            self.status_page.set_icon_name(Some(config::APP_ID));
            page.reset_validation();
            // Reset the validation whenever the password state changes
            page.connect_has_set_password_notify(clone!(@weak page  => move |_| {
                page.reset_validation();
            }));
        }
    }

    impl WidgetImpl for PasswordPage {
        fn unmap(&self) {
            self.parent_unmap();
            self.obj().reset();
        }
    }

    impl BoxImpl for PasswordPage {}
}

glib::wrapper! {
    pub struct PasswordPage(ObjectSubclass<imp::PasswordPage>)
        @extends gtk::Widget, gtk::Box;
}

#[gtk::template_callbacks]
impl PasswordPage {
    pub fn new(actions: gio::SimpleActionGroup) -> Self {
        let page = glib::Object::new::<Self>();
        page.imp().actions.set(actions).unwrap();
        page.setup_actions();
        page
    }

    #[template_callback]
    fn validate(&self, _entry: Option<gtk::Editable>) {
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

    // Called when either the user sets/resets the password to bind/unbind the
    // the validation callback on the password entry
    fn reset_validation(&self) {
        let imp = self.imp();
        if self.has_set_password() {
            imp.current_password_entry
                .connect_changed(clone!(@weak self as page => move |_| page.validate(None)));
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
                let ctx = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak page => async move {
                    page.save().await;
                }));
            })
        );

        action!(
            actions,
            "reset_password",
            clone!(@weak self as page => move |_,_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak page => async move {
                    page.reset_password().await;
                }));
            })
        );
        get_action!(actions, @save_password).set_enabled(false);

        self.bind_property(
            "has-set-password",
            &get_action!(actions, @reset_password),
            "enabled",
        )
        .sync_create()
        .build();
    }

    async fn reset_password(&self) {
        let imp = self.imp();

        let current_password = imp.current_password_entry.text();
        let is_current_password =
            spawn_tokio(async move { keyring::is_current_password(&current_password).await })
                .await
                .unwrap_or(false);
        if self.has_set_password() && !is_current_password {
            imp.error_revealer.popup(&gettext("Wrong Passphrase"));
            return;
        }

        let password_was_reset =
            spawn_tokio(async move { keyring::reset_password().await.is_ok() }).await;

        if password_was_reset {
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

    async fn save(&self) {
        let imp = self.imp();
        let actions = imp.actions.get().unwrap();

        let current_password = imp.current_password_entry.text();
        let password = imp.password_entry.text();
        let is_current_password = spawn_tokio(async move {
            keyring::is_current_password(&current_password)
                .await
                .unwrap_or(false)
        })
        .await;

        if self.has_set_password() && is_current_password {
            imp.error_revealer.popup(&gettext("Wrong Passphrase"));
            return;
        }
        let password_was_set =
            spawn_tokio(async move { keyring::set_password(&password).await.is_ok() }).await;
        if password_was_set {
            self.reset();
            get_action!(actions, @save_password).set_enabled(false);
            self.set_has_set_password(true);
            get_action!(actions, @close_page).activate(None);
        }
    }
}
