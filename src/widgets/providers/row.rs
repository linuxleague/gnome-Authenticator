use std::time::{SystemTime, UNIX_EPOCH};

use adw::prelude::*;
use gtk::{glib, glib::clone, subclass::prelude::*, CompositeTemplate};

use crate::{
    models::{Account, AccountSorter, OTPMethod, Provider},
    widgets::{accounts::AccountRow, ProgressIcon, ProviderImage},
};

mod imp {
    use std::cell::RefCell;

    use glib::{
        subclass::{self, Signal},
        ParamFlags, ParamSpec, ParamSpecObject, Value,
    };
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_row.ui")]
    pub struct ProviderRow {
        pub provider: RefCell<Option<Provider>>,
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

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecObject::new(
                    "provider",
                    "",
                    "",
                    Provider::static_type(),
                    ParamFlags::READWRITE | ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("changed", &[], <()>::static_type().into())
                        .action()
                        .build(),
                    Signal::builder(
                        "shared",
                        &[Account::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .action()
                    .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "provider" => self.provider.borrow().to_value(),
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
        self.property("provider")
    }

    fn tick_progressbar(&self) {
        let imp = self.imp();
        let period_millis = self.provider().period() as u128 * 1000;
        let now: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let remaining_time: u128 = period_millis - now % period_millis;

        let progress_fraction: f64 = (remaining_time as f64) / (period_millis as f64);

        imp.progress_icon.set_progress(progress_fraction as f32);
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        let provider = self.provider();

        self.add_css_class(&provider.method().to_string());

        imp.image.set_provider(Some(&provider));
        if provider.method() == OTPMethod::HOTP {
            imp.progress_icon.hide();
        } else {
            // Update the progress bar whnever the remaining-time is updated
            self.tick_progressbar();
            provider.connect_notify_local(
                Some("remaining-time"),
                clone!(@weak self as row => move |_, _| {
                    row.tick_progressbar();
                }),
            );
        }

        provider
            .bind_property("name", &*imp.name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        let sorter = AccountSorter::default();
        let sort_model = gtk::SortListModel::new(Some(provider.accounts()), Some(&sorter));

        let create_callback = clone!(@strong self as provider_row, @strong sorter, @strong provider => move |account: &glib::Object| {
            let account = account.clone().downcast::<Account>().unwrap();
            let row = AccountRow::new(account.clone());

            row.connect_activated(
                clone!(@weak account, @weak provider_row => move |_| {
                    provider_row.emit_by_name::<()>("shared", &[&account]);
                }),
            );

            account.connect_local("notify::name",
                false,
                clone!(@weak provider_row, @weak sorter => @default-return None, move |_| {
                    // Re-sort in case the name was updated
                    sorter.changed(gtk::SorterChange::Different);
                    provider_row.emit_by_name::<()>("changed", &[]);
                    None
                }),
            );
            row.upcast::<gtk::Widget>()
        });

        imp.accounts_list
            .bind_model(Some(&sort_model), create_callback);
    }
}
