use crate::models::Account;
use glib::clone;
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use std::cell::RefCell;

mod imp {
    use super::*;
    use glib::{
        subclass::{self, Signal},
        ParamSpec,
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_row.ui")]
    pub struct AccountRow {
        pub account: RefCell<Option<Account>>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub edit_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub otp_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountRow {
        const NAME: &'static str = "AccountRow";
        type Type = super::AccountRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("removed", &[], <()>::static_type().into())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                    Signal::builder("shared", &[], <()>::static_type().into())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpec::new_object(
                    "account",
                    "Account",
                    "The account",
                    Account::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }
        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "account" => {
                    let account = value.get().unwrap();
                    self.account.replace(account);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "account" => self.account.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_actions();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountRow {}
    impl ListBoxRowImpl for AccountRow {}
}

glib::wrapper! {
    pub struct AccountRow(ObjectSubclass<imp::AccountRow>) @extends gtk::Widget, gtk::ListBoxRow;
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
        self.connect_activate(move |row| {
            row.activate_action("account.details", None);
        });

        self.account()
            .bind_property("name", &*imp.name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("name", &*imp.name_label, "tooltip-text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("name", &*imp.name_entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("otp", &*imp.otp_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        imp
            .name_entry
            .connect_changed(clone!(@weak imp.actions as actions => move |entry| {
                let name = entry.text();
                get_action!(actions, @save).set_enabled(!name.is_empty());
            }));
        imp
            .name_entry
            .connect_activate(clone!(@weak imp.actions as actions => move |_| {
                   actions.activate_action("save", None);
            }));
    }

    fn setup_actions(&self) {
        let imp = self.imp();
        self.insert_action_group("account", Some(&imp.actions));
        action!(
            imp.actions,
            "copy-otp",
            clone!(@weak self as row => move |_, _| {
                row.account().copy_otp();
            })
        );

        action!(
            imp.actions,
            "details",
            clone!(@weak self as row => move |_, _| {
                row.emit_by_name("shared", &[]);
            })
        );

        action!(
            imp.actions,
            "delete",
            clone!(@weak self as row => move |_, _| {
                row.emit_by_name("removed", &[]);
            })
        );

        let edit_stack = &*imp.edit_stack;
        let name_entry = &*imp.name_entry;
        action!(
            imp.actions,
            "rename",
            clone!(@weak edit_stack, @weak name_entry => move |_, _| {
                edit_stack.set_visible_child_name("edit");
                name_entry.grab_focus();
            })
        );

        action!(
            imp.actions,
            "save",
            clone!(@weak self as row, @weak edit_stack, @weak name_entry => move |_, _| {
                let new_name = name_entry.text();
                if let Err(err) = row.account().set_name(&new_name) {
                    error!("Failed to update the account name {}", err);
                }
                edit_stack.set_visible_child_name("display");
            })
        );
    }
}
