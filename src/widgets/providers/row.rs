use crate::models::{Account, AccountSorter, Algorithm, Provider};
use crate::widgets::{accounts::AccountRow, ProviderImage, ProviderImageSize};
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
        pub image: ProviderImage,
        pub provider: RefCell<Option<Provider>>,
        #[template_child(id = "name_label")]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child(id = "accounts_list")]
        pub accounts_list: TemplateChild<gtk::ListBox>,
        #[template_child(id = "progress")]
        pub progress: TemplateChild<gtk::ProgressBar>,
        #[template_child(id = "header")]
        pub header: TemplateChild<gtk::Box>,
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
                image: ProviderImage::new(ProviderImageSize::Small),
                name_label: TemplateChild::default(),
                accounts_list: TemplateChild::default(),
                progress: TemplateChild::default(),
                header: TemplateChild::default(),
                provider: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/provider_row.ui");
            Self::bind_template_children(klass);
            klass.install_properties(&PROPERTIES);
            klass.add_signal("changed", glib::SignalFlags::ACTION, &[], glib::Type::Unit);
            klass.add_signal(
                "shared",
                glib::SignalFlags::ACTION,
                &[Account::static_type()],
                glib::Type::Unit,
            );
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

        self.add_css_class(&self.provider().algorithm().to_string());

        self_.header.get().prepend(&self_.image);
        self_.image.set_provider(&self.provider());

        let progress_bar = self_.progress.get();
        if self.provider().algorithm() == Algorithm::TOTP {
            progress_bar.set_fraction(1_f64);
            let max = self.provider().period() as f64;
            glib::timeout_add_local(
                std::time::Duration::from_millis(50),
                clone!(@weak progress_bar => @default-return glib::Continue(false), move || {
                    let mut new_value = progress_bar.get_fraction() - (0.05/max);
                    if new_value <= 0.0 {
                        new_value = 1.0;
                    }
                    progress_bar.set_fraction(new_value);
                    glib::Continue(true)
                }),
            );
        } else {
            progress_bar.hide();
        }

        self.provider()
            .bind_property("name", &self_.name_label.get(), "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        let sorter = AccountSorter::new();
        let sort_model = gtk::SortListModel::new(Some(self.provider().accounts()), Some(&sorter));

        let provider = self.provider();

        let create_callback = clone!(@weak self as provider_row, @weak sorter, @weak provider => move |account: &glib::Object| {
            let account = account.clone().downcast::<Account>().unwrap();
            let row = AccountRow::new(account.clone());
            row.connect_local(
                "removed",
                false,
                clone!(@weak provider, @weak account, @weak provider_row => move |_| {
                    account.delete().unwrap();
                    provider.remove_account(account);
                    provider_row.emit("changed", &[]).unwrap();
                    None
                }),
            ).unwrap();

            row.connect_local(
                "shared",
                false,
                clone!(@weak account, @weak provider_row => move |_| {
                    provider_row.emit("shared", &[&account]).unwrap();
                    None
                }),
            ).unwrap();

            account.connect_local("notify::name",
                false,
                clone!(@weak provider_row, @weak sorter => move |_| {
                    // Re-sort in case the name was updated
                    sorter.changed(gtk::SorterChange::Different);
                    provider_row.emit("changed", &[]).unwrap();
                    None
                }),
            )
            .unwrap();
            row.upcast::<gtk::Widget>()
        });

        self_
            .accounts_list
            .get()
            .bind_model(Some(&sort_model), Some(Box::new(create_callback)));
    }
}
