use crate::models::{Account, OTPMethod};
use gtk::{gdk, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use std::cell::RefCell;

mod imp {
    use crate::widgets::Window;

    use super::*;
    use adw::subclass::prelude::*;
    use gettextrs::gettext;
    use glib::{subclass, ParamSpec, ParamSpecObject, Value};
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_row.ui")]
    pub struct AccountRow {
        pub account: RefCell<Option<Account>>,
        #[template_child]
        pub increment_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub otp_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountRow {
        const NAME: &'static str = "AccountRow";
        type Type = super::AccountRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(
                gdk::Key::c,
                gdk::ModifierType::CONTROL_MASK,
                "account.copy-otp",
                None,
            );

            klass.install_action("account.copy-otp", None, move |row, _, _| {
                row.account().copy_otp();
                let window = row.root().unwrap().downcast::<Window>().unwrap();
                let toast = adw::Toast::new(&gettext("One-Time password copied"));
                toast.set_timeout(3);
                window.add_toast(toast);
            });
            klass.install_action("account.increment-counter", None, move |row, _, _| {
                match row.account().increment_counter() {
                    Ok(_) => row.account().generate_otp(),
                    Err(err) => log::error!("Failed to increment the counter {err}"),
                };
            });
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecObject::new(
                    "account",
                    "Account",
                    "The account",
                    Account::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }
        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "account" => {
                    let account = value.get().unwrap();
                    self.account.replace(account);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "account" => self.account.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountRow {}
    impl ListBoxRowImpl for AccountRow {}
    impl PreferencesRowImpl for AccountRow {}
    impl ActionRowImpl for AccountRow {}
}

glib::wrapper! {
    pub struct AccountRow(ObjectSubclass<imp::AccountRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl AccountRow {
    pub fn new(account: Account) -> Self {
        glib::Object::new(&[("account", &account)]).expect("Failed to create AccountRow")
    }

    fn account(&self) -> Account {
        self.property("account")
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        let account = self.account();
        account
            .bind_property("name", self, "title")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        account
            .bind_property("name", self, "tooltip-text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        account
            .bind_property("otp", &*imp.otp_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        // Only display the increment button if it is a HOTP account
        imp.increment_btn
            .set_visible(account.provider().method() == OTPMethod::HOTP);
    }
}
