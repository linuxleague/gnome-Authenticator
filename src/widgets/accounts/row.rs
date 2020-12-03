use crate::models::Account;
use gio::prelude::*;
use gio::{subclass::ObjectSubclass, ActionMapExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use std::cell::RefCell;
mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

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
        #[template_child(id = "name_label")]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child(id = "name_entry")]
        pub name_entry: TemplateChild<gtk::Entry>,
        #[template_child(id = "edit_stack")]
        pub edit_stack: TemplateChild<gtk::Stack>,
    }

    impl ObjectSubclass for AccountRow {
        const NAME: &'static str = "AccountRow";
        type Type = super::AccountRow;
        type ParentType = gtk::ListBoxRow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let actions = gio::SimpleActionGroup::new();

            Self {
                actions,
                name_label: TemplateChild::default(),
                name_entry: TemplateChild::default(),
                edit_stack: TemplateChild::default(),
                account: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/account_row.ui");
            Self::bind_template_children(klass);
            klass.install_properties(&PROPERTIES);
        }
    }

    impl ObjectImpl for AccountRow {
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("account", ..) => {
                    let account = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
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
            obj.init_template();
            obj.setup_actions();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountRow {}
    impl ListBoxRowImpl for AccountRow {}
}

glib_wrapper! {
    pub struct AccountRow(ObjectSubclass<imp::AccountRow>) @extends gtk::Widget, gtk::ListBoxRow;
}

impl AccountRow {
    pub fn new(account: Account) -> Self {
        glib::Object::new(Self::static_type(), &[("account", &account)])
            .expect("Failed to create AccountRow")
            .downcast::<AccountRow>()
            .expect("Created object is of wrong type")
    }

    fn account(&self) -> Account {
        let account = self.get_property("account").unwrap();
        account.get::<Account>().unwrap().unwrap()
    }

    fn setup_widgets(&self) {
        let self_ = imp::AccountRow::from_instance(self);
        self.account()
            .bind_property("name", &self_.name_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account()
            .bind_property("name", &self_.name_entry.get(), "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self_.name_entry.get().connect_changed(
            clone!(@strong self_.actions as actions => move |entry| {
                let name = entry.get_text().unwrap();
                get_action!(actions, @save).set_enabled(!name.is_empty());
            }),
        );
    }

    fn setup_actions(&self) {
        let self_ = imp::AccountRow::from_instance(self);
        self.insert_action_group("account", Some(&self_.actions));
        action!(self_.actions, "delete", move |_, _| {
            //send!(sender, Action::AccountRemoved(account.clone()));
        });

        let edit_stack = self_.edit_stack.get();
        action!(
            self_.actions,
            "edit",
            clone!(@weak edit_stack => move |_, _| {
                edit_stack.set_visible_child_name("edit");
            })
        );

        let name_entry = self_.name_entry.get();
        action!(
            self_.actions,
            "save",
            clone!(@weak edit_stack, @weak name_entry => move |_, _| {
                let new_name = name_entry.get_text().unwrap();

                edit_stack.set_visible_child_name("display");
            })
        );
    }
}
