use crate::{
    models::{Account, AccountSorter, OTPMethod, Provider},
    widgets::{accounts::AccountRow, ProviderImage},
};
use gtk::subclass::prelude::*;
use gtk::{glib, glib::clone, prelude::*, CompositeTemplate};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod imp {
    use super::*;
    use glib::subclass;
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

        glib::object_subclass!();

        fn new() -> Self {
            Self {
                remaining_time: Cell::new(0),
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

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
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
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProviderRow {}
    impl ListBoxRowImpl for ProviderRow {}
}

glib::wrapper! {
    pub struct ProviderRow(ObjectSubclass<imp::ProviderRow>) @extends gtk::Widget, gtk::ListBoxRow;
}

impl ProviderRow {
    pub fn new(provider: Provider) -> Self {
        glib::Object::new(&[("provider", &provider)]).expect("Failed to create ProviderRow")
    }

    fn provider(&self) -> Provider {
        let provider = self.get_property("provider").unwrap();
        provider.get::<Provider>().unwrap().unwrap()
    }

    fn restart(&self) {
        let provider = self.provider();

        if provider.method() == OTPMethod::TOTP {
            let self_ = imp::ProviderRow::from_instance(self);

            // If current_time is writen as 30 * x + r, where r
            // is the integer such that 0<= r < 30, this returns 30 * x.
            let last_epoch: u64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                % self.provider().period() as u64
                * self.provider().period() as u64;

            self_.progress.set_fraction(1_f64);
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
        let remaining_time: u64 = self.provider().period() as u64
            - SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                % self.provider().period() as u64;

        self.set_property("remaining-time", &remaining_time)
            .unwrap();
    }

    fn tick_progressbar(&self) {
        let self_ = imp::ProviderRow::from_instance(self);
        let period_millis = self.provider().period() as u128 * 1000;
        let now: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let remaining_time: u128 = period_millis - now % period_millis;

        let progress_fraction: f64 = (remaining_time as f64) / (period_millis as f64);

        self_.progress.set_fraction(progress_fraction);
        // TODO This can be improved, as the time window is big enough
        // so that restart will be called multiples times. 0.002 correspods to
        // about 16 frames, this callback won't be run on machines which can't display
        // more than 16 frames.
        if (progress_fraction - 1.0).abs() < 0.002 {
            self.restart();
        }
        // When there is left than 1/5 of the time remaining, turn the bar red.
        if progress_fraction < 0.2 {
            self_.progress.add_css_class("red-progress")
        } else {
            self_.progress.remove_css_class("red-progress")
        }
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderRow::from_instance(self);

        self.add_css_class(&self.provider().method().to_string());

        self_.image.set_provider(&self.provider());

        self.restart();
        if self.provider().method() == OTPMethod::TOTP {
            glib::timeout_add_seconds_local(
                1,
                clone!(@weak self as row => @default-return glib::Continue(false), move || {
                    row.tick();
                    glib::Continue(true)
                }),
            );

            self.add_tick_callback(|row, _| {
                row.tick_progressbar();
                glib::Continue(true)
            });
        } else {
            self_.progress.hide();
        }

        self.provider()
            .bind_property("name", &*self_.name_label, "label")
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
            .bind_model(Some(&sort_model), create_callback);
    }
}
