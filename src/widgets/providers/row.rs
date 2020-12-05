use crate::models::{Account, AccountSorter, Provider};
use crate::widgets::accounts::AccountRow;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use std::cell::RefCell;
mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    static PROPERTIES: [subclass::Property; 1] = [subclass::Property("provider", |name| {
        glib::ParamSpec::object(
            name,
            "Provider",
            "The accounts provider",
            Provider::static_type(),
            glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
        )
    })];

    #[derive(CompositeTemplate)]
    pub struct ProviderRow {
        pub provider: RefCell<Option<Provider>>,
        #[template_child(id = "name_label")]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child(id = "accounts_list")]
        pub accounts_list: TemplateChild<gtk::ListBox>,
    }

    impl ObjectSubclass for ProviderRow {
        const NAME: &'static str = "ProviderRow";
        type Type = super::ProviderRow;
        type ParentType = gtk::ListBoxRow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            Self {
                name_label: TemplateChild::default(),
                accounts_list: TemplateChild::default(),
                provider: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/provider_row.ui");
            Self::bind_template_children(klass);
            klass.install_properties(&PROPERTIES);
        }
    }

    impl ObjectImpl for ProviderRow {
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("provider", ..) => {
                    let provider = value
                        .get()
                        .expect("type conformity checked by `Object::set_property`");
                    self.provider.replace(provider);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];
            match *prop {
                subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProviderRow {}
    impl ListBoxRowImpl for ProviderRow {}
}

glib_wrapper! {
    pub struct ProviderRow(ObjectSubclass<imp::ProviderRow>) @extends gtk::Widget, gtk::ListBoxRow;
}

impl ProviderRow {
    pub fn new(provider: Provider) -> Self {
        glib::Object::new(Self::static_type(), &[("provider", &provider)])
            .expect("Failed to create ProviderRow")
            .downcast::<ProviderRow>()
            .expect("Created object is of wrong type")
    }

    fn provider(&self) -> Provider {
        let provider = self.get_property("provider").unwrap();
        provider.get::<Provider>().unwrap().unwrap()
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderRow::from_instance(self);

        self.provider()
            .bind_property("name", &self_.name_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        let sorter = AccountSorter::new();
        let sort_model = gtk::SortListModel::new(Some(self.provider().accounts()), Some(&sorter));

        self_.accounts_list.get().bind_model(
            Some(&sort_model),
            Some(Box::new(move |account: &glib::Object| {
                let account = account.clone().downcast::<Account>().unwrap();
                AccountRow::new(account).upcast::<gtk::Widget>()
            })),
        );
    }
}
