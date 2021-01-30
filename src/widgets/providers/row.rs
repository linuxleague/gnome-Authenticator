use crate::{
    models::{Account, AccountSorter, OTPMethod, Provider},
    widgets::{accounts::AccountRow, ProviderImage},
};
use gtk::{glib, glib::clone, prelude::*, subclass::prelude::*, CompositeTemplate};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod imp {
    use super::*;
    use glib::{
        subclass::{self, Signal},
        ParamSpec,
    };
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_row.ui")]
    pub struct ProviderRow {
        pub remaining_time: Cell<u64>,
        pub provider: RefCell<Option<Provider>>,
        pub callback_id: RefCell<Option<gtk::TickCallbackId>>,
        pub schedule: RefCell<Option<glib::SourceId>>,
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
        type Interfaces = ();
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
                callback_id: RefCell::default(),
                schedule: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::object(
                        "provider",
                        "Provider",
                        "The accounts provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    ParamSpec::uint64(
                        "remaining-time",
                        "remaining time",
                        "the remaining time",
                        0,
                        u64::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("changed", &[], <()>::static_type())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                    Signal::builder("shared", &[Account::static_type()], <()>::static_type())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.get_name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                "remaining-time" => {
                    let remaining_time = value.get().unwrap().unwrap();
                    self.remaining_time.set(remaining_time);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "provider" => self.provider.borrow().to_value(),
                "remaining-time" => self.remaining_time.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            self.parent_constructed(obj);
        }

        fn dispose(&self, _obj: &Self::Type) {
            if let Some(id) = self.callback_id.borrow_mut().take() {
                id.remove();
            }
            if let Some(id) = self.schedule.borrow_mut().take() {
                glib::source_remove(id);
            }
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

        match self.provider().method() {
            OTPMethod::TOTP | OTPMethod::Steam => {
                let self_ = imp::ProviderRow::from_instance(self);

                self_.progress.set_fraction(1_f64);
                self.set_property("remaining-time", &(self.provider().period() as u64))
                    .unwrap();
            }
            _ => (),
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
        if remaining_time <= 1000 && self_.schedule.borrow().is_none() {
            let id = glib::timeout_add_local(
                Duration::from_millis(remaining_time as u64),
                clone!(@weak self as row  => @default-return glib::Continue(false), move || {
                    row.restart();
                    let row_ = imp::ProviderRow::from_instance(&row);
                    row_.schedule.replace(None);

                    glib::Continue(false)
                }),
            );
            self_.schedule.replace(Some(id));
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

        self_.image.set_provider(Some(&self.provider()));

        self.restart();
        match self.provider().method() {
            OTPMethod::TOTP | OTPMethod::Steam => {
                glib::timeout_add_seconds_local(
                    1,
                    clone!(@weak self as row => @default-return glib::Continue(false), move || {
                        row.tick();
                        glib::Continue(true)
                    }),
                );

                self_
                    .callback_id
                    .replace(Some(self.add_tick_callback(|row, _| {
                        row.tick_progressbar();
                        glib::Continue(true)
                    })));
            }
            _ => self_.progress.hide(),
        }

        self.provider()
            .bind_property("name", &*self_.name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        let sorter = AccountSorter::new();
        let sort_model = gtk::SortListModel::new(Some(self.provider().accounts()), Some(&sorter));

        let provider = self.provider();

        let create_callback = clone!(@strong self as provider_row, @weak sorter, @weak provider => move |account: &glib::Object| {
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
