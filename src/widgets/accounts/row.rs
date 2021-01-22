use crate::models::Account;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use std::cell::RefCell;

mod imp {
    use super::*;
    use glib::subclass;

    static PROPERTIES: [subclass::Property; 1] = [subclass::Property("account", |name| {
        glib::ParamSpec::object(
            name,
            "Account",
            "The account",
            Account::static_type(),
            glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
        )
    })];

    #[derive(CompositeTemplate)]
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
        pub copy_btn_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub otp_label: TemplateChild<gtk::Label>,
    }

    impl ObjectSubclass for AccountRow {
        const NAME: &'static str = "AccountRow";
        type Type = super::AccountRow;
        type ParentType = gtk::ListBoxRow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let actions = gio::SimpleActionGroup::new();

            Self {
                actions,
                name_label: TemplateChild::default(),
                name_entry: TemplateChild::default(),
                edit_stack: TemplateChild::default(),
                otp_label: TemplateChild::default(),
                copy_btn_stack: TemplateChild::default(),
                account: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/account_row.ui");
            Self::bind_template_children(klass);
            klass.install_properties(&PROPERTIES);
            klass.add_signal("removed", glib::SignalFlags::ACTION, &[], glib::Type::Unit);
            klass.add_signal("shared", glib::SignalFlags::ACTION, &[], glib::Type::Unit);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountRow {
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("account", ..) => {
                    let account = value.get().unwrap();
                    self.account.replace(account);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];
            match *prop {
                subclass::Property("account", ..) => self.account.borrow().to_value(),
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
        let account = self.get_property("account").unwrap();
        account.get::<Account>().unwrap().unwrap()
    }

    fn setup_widgets(&self) {
        let self_ = imp::AccountRow::from_instance(self);
        self.account()
            .bind_property("name", &*self_.name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("name", &*self_.name_entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("otp", &*self_.otp_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self_
            .name_entry
            .connect_changed(clone!(@weak self_.actions as actions => move |entry| {
                let name = entry.get_text().unwrap();
                get_action!(actions, @save).set_enabled(!name.is_empty());
            }));
        self_
            .name_entry
            .connect_activate(clone!(@weak self_.actions as actions => move |_| {
                   actions.activate_action("save", None);
            }));
    }

    fn setup_actions(&self) {
        let self_ = imp::AccountRow::from_instance(self);
        self.insert_action_group("account", Some(&self_.actions));
        let copy_btn_stack = &*self_.copy_btn_stack;
        action!(
            self_.actions,
            "copy-otp",
            clone!(@weak self as row, @weak copy_btn_stack => move |_, _| {
                copy_btn_stack.set_visible_child_name("ok");
                row.account().copy_otp();
                glib::source::timeout_add_seconds_local(1, clone!(@weak copy_btn_stack => move || {
                    copy_btn_stack.set_visible_child_name("copy");
                    glib::Continue(false)
                }));
            })
        );

        action!(
            self_.actions,
            "details",
            clone!(@weak self as row => move |_, _| {
                row.emit("shared", &[]).unwrap();
            })
        );

        action!(
            self_.actions,
            "delete",
            clone!(@weak self as row => move |_, _| {
                row.emit("removed", &[]).unwrap();
            })
        );

        let edit_stack = &*self_.edit_stack;
        let name_entry = &*self_.name_entry;
        action!(
            self_.actions,
            "rename",
            clone!(@weak edit_stack, @weak name_entry => move |_, _| {
                edit_stack.set_visible_child_name("edit");
                name_entry.grab_focus();
            })
        );

        action!(
            self_.actions,
            "save",
            clone!(@weak self as row, @weak edit_stack, @weak name_entry => move |_, _| {
                let new_name = name_entry.get_text().unwrap();
                if let Err(err) = row.account().set_name(&new_name) {
                    error!("Failed to update the account name {}", err);
                }
                edit_stack.set_visible_child_name("display");
            })
        );
    }
}
