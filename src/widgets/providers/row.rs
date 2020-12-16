use crate::{
    models::{Account, AccountSorter, OTPMethod, Provider},
    widgets::{accounts::AccountRow, ProviderImage},
};
use gio::{prelude::*, subclass::ObjectSubclass};
use glib::{clone, glib_object_subclass, glib_wrapper, subclass::prelude::*};
use gtk::{prelude::*, CompositeTemplate};
use std::time::{Duration, Instant};

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    static PROPERTIES: [subclass::Property; 2] = [
        subclass::Property("provider", |name| {
            glib::ParamSpec::object(
                name,
                "Provider",
                "The accounts provider",
                Provider::static_type(),
                glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
            )
        }),
        subclass::Property("remaining-time", |name| {
            glib::ParamSpec::uint64(
                name,
                "remaining time",
                "the remaining time",
                0,
                u64::MAX,
                0,
                glib::ParamFlags::READWRITE,
            )
        }),
    ];

    #[derive(CompositeTemplate)]
    pub struct ProviderRow {
        pub remaining_time: Cell<u64>,
        pub started_at: RefCell<Option<Instant>>,
        pub provider: RefCell<Option<Provider>>,
        #[template_child]
        pub image: TemplateChild<ProviderImage>,
        #[template_child]
        pub name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub accounts_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub progress: TemplateChild<gtk::ProgressBar>,
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
                remaining_time: Cell::new(0),
                started_at: RefCell::new(None),
                image: TemplateChild::default(),
                name_label: TemplateChild::default(),
                accounts_list: TemplateChild::default(),
                progress: TemplateChild::default(),
                provider: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            ProviderImage::static_type();
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
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                subclass::Property("remaining-time", ..) => {
                    let remaining_time = value.get().unwrap().unwrap();
                    self.remaining_time.set(remaining_time);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];
            match *prop {
                subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                subclass::Property("remaining-time", ..) => self.remaining_time.get().to_value(),
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

    fn restart(&self) {
        let provider = self.provider();

        if provider.method() == OTPMethod::TOTP {
            let self_ = imp::ProviderRow::from_instance(self);

            self_.started_at.borrow_mut().replace(Instant::now());
            self_.progress.get().set_fraction(1_f64);
            self.set_property("remaining-time", &(self.provider().period() as u64))
                .unwrap();
        }

        // Tell all of the accounts to regen
        let accounts = provider.accounts();
        for i in 0..accounts.get_n_items() {
            let item = accounts.get_object(i).unwrap();
            let account = item.downcast_ref::<Account>().unwrap();
            account.generate_otp();
        }
    }

    fn tick(&self) {
        let self_ = imp::ProviderRow::from_instance(self);
        let max = self.provider().period() as f64;
        let started_at = self_.started_at.borrow().clone().unwrap();
        let remaining_time = started_at.elapsed().as_secs();

        self.set_property("remaining-time", &remaining_time)
            .unwrap();
    }

    fn tick_progressbar(&self) {
        let self_ = imp::ProviderRow::from_instance(self);
        let max = 1000_f64 * self.provider().period() as f64;

        let started_at = self_.started_at.borrow().clone().unwrap();
        let remaining_time = started_at.elapsed().as_millis();
        let progress_fraction = (max - (remaining_time as f64)) / max;

        self_.progress.get().set_fraction(progress_fraction);
        if progress_fraction <= 0.0 {
            self.restart();
        }
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderRow::from_instance(self);

        self.add_css_class(&self.provider().method().to_string());

        self_.image.get().set_provider(&self.provider());

        self.restart();
        if self.provider().method() == OTPMethod::TOTP {
            glib::timeout_add_seconds_local(
                1,
                clone!(@weak self as row => @default-return glib::Continue(false), move || {
                    row.tick();
                    glib::Continue(true)
                }),
            );

            glib::timeout_add_local(
                Duration::from_millis(20),
                clone!(@weak self as row => @default-return glib::Continue(false), move || {
                    row.tick_progressbar();
                    glib::Continue(true)
                }),
            );
        } else {
            self_.progress.get().hide();
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
