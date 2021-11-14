use crate::{
    models::{Account, AccountSorter, OTPMethod, Provider},
    widgets::{accounts::AccountRow, ProviderImage, ProgressIcon, ProgressIconExt},
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
        pub progress_icon: TemplateChild<ProgressIcon>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProviderRow {
        const NAME: &'static str = "ProviderRow";
        type Type = super::ProviderRow;
        type ParentType = gtk::ListBoxRow;

        fn new() -> Self {
            Self {
                remaining_time: Cell::new(0),
                image: TemplateChild::default(),
                name_label: TemplateChild::default(),
                accounts_list: TemplateChild::default(),
                progress_icon: TemplateChild::default(),
                provider: RefCell::new(None),
                callback_id: RefCell::default(),
                schedule: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_object(
                        "provider",
                        "Provider",
                        "The accounts provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    ParamSpec::new_uint64(
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
                    Signal::builder("changed", &[], <()>::static_type().into())
                        .flags(glib::SignalFlags::ACTION)
                        .build(),
                    Signal::builder(
                        "shared",
                        &[Account::static_type().into()],
                        <()>::static_type().into(),
                    )
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
            match pspec.name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                "remaining-time" => {
                    let remaining_time = value.get().unwrap();
                    self.remaining_time.set(remaining_time);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
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
                id.remove();
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
        self.property("provider")
    }

    fn restart(&self) {
        let provider = self.provider();

        match provider.method() {
            OTPMethod::TOTP | OTPMethod::Steam => {
                let self_ = imp::ProviderRow::from_instance(self);

                self_.progress_icon.set_progress(1_f32);
                self.set_property("remaining-time", &(provider.period() as u64));
            }
            _ => (),
        }

        // Tell all of the accounts to regen
        let accounts = provider.accounts();
        for i in 0..accounts.n_items() {
            let item = accounts.item(i).unwrap();
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

        self.set_property("remaining-time", &remaining_time);
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

        self_.progress_icon.set_progress(progress_fraction as f32);
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
            _ => self_.progress_icon.hide(),
        }

        self.provider()
            .bind_property("name", &*self_.name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        let sorter = AccountSorter::new();
        let sort_model = gtk::SortListModel::new(Some(self.provider().accounts()), Some(&sorter));

        let provider = self.provider();

        let create_callback = clone!(@strong self as provider_row, @strong sorter, @strong provider => move |account: &glib::Object| {
            let account = account.clone().downcast::<Account>().unwrap();
            let row = AccountRow::new(account.clone());
            row.connect_local(
                "removed",
                false,
                clone!(@weak provider, @weak account, @weak provider_row => @default-return None, move |_| {
                    account.delete().unwrap();
                    provider.remove_account(account);
                    provider_row.emit_by_name("changed", &[]);
                    None
                }),
            );

            row.connect_local(
                "shared",
                false,
                clone!(@weak account, @weak provider_row => @default-return None, move |_| {
                    provider_row.emit_by_name("shared", &[&account]);
                    None
                }),
            );

            account.connect_local("notify::name",
                false,
                clone!(@weak provider_row, @weak sorter => @default-return None, move |_| {
                    // Re-sort in case the name was updated
                    sorter.changed(gtk::SorterChange::Different);
                    provider_row.emit_by_name("changed", &[]);
                    None
                }),
            );
            row.upcast::<gtk::Widget>()
        });

        self_
            .accounts_list
            .bind_model(Some(&sort_model), create_callback);
    }
}
