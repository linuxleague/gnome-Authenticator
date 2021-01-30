use crate::{
    models::{i18n, otp, Algorithm, OTPMethod, Provider},
    widgets::ProviderImage,
};
use adw::ComboRowExt;
use gettextrs::gettext;
use glib::{clone, translate::ToGlib};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};

mod imp {
    use super::*;
    use crate::models::OTPMethod;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_page.ui")]
    pub struct ProviderPage {
        pub actions: gio::SimpleActionGroup,
        pub methods_model: adw::EnumListModel,
        pub algorithms_model: adw::EnumListModel,
        #[template_child]
        pub image: TemplateChild<ProviderImage>,
        #[template_child]
        pub name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub period_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub digits_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub default_counter_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub provider_website_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub provider_help_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub method_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub algorithm_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub period_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub digits_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub default_counter_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        pub selected_provider: RefCell<Option<Provider>>,
    }

    impl ObjectSubclass for ProviderPage {
        const NAME: &'static str = "ProviderPage";
        type Type = super::ProviderPage;
        type ParentType = gtk::Box;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let methods_model = adw::EnumListModel::new(OTPMethod::static_type());
            let algorithms_model = adw::EnumListModel::new(Algorithm::static_type());

            Self {
                actions: gio::SimpleActionGroup::new(),
                image: TemplateChild::default(),
                name_entry: TemplateChild::default(),
                period_spinbutton: TemplateChild::default(),
                digits_spinbutton: TemplateChild::default(),
                default_counter_spinbutton: TemplateChild::default(),
                provider_website_entry: TemplateChild::default(),
                provider_help_entry: TemplateChild::default(),
                method_comborow: TemplateChild::default(),
                algorithm_comborow: TemplateChild::default(),
                period_row: TemplateChild::default(),
                digits_row: TemplateChild::default(),
                default_counter_row: TemplateChild::default(),
                title: TemplateChild::default(),
                methods_model,
                algorithms_model,
                selected_provider: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderPage {
        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("created", &[Provider::static_type()], <()>::static_type())
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            obj.setup_actions();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProviderPage {}
    impl BoxImpl for ProviderPage {}
}

glib::wrapper! {
    pub struct ProviderPage(ObjectSubclass<imp::ProviderPage>) @extends gtk::Widget, gtk::Box;
}
impl ProviderPage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderPage")
    }

    pub fn set_provider(&self, provider: Option<Provider>) {
        let self_ = imp::ProviderPage::from_instance(self);
        if let Some(provider) = provider {
            self_.name_entry.set_text(&provider.name());
            self_.period_spinbutton.set_value(provider.period() as f64);

            if let Some(ref website) = provider.website() {
                self_.provider_website_entry.set_text(website);
            }

            if let Some(ref website) = provider.help_url() {
                self_.provider_help_entry.set_text(website);
            }

            self_.algorithm_comborow.set_selected(
                self_
                    .algorithms_model
                    .find_position(provider.algorithm().to_glib()),
            );

            self_
                .default_counter_spinbutton
                .set_value(provider.default_counter() as f64);
            self_.digits_spinbutton.set_value(provider.digits() as f64);

            self_.method_comborow.set_selected(
                self_
                    .methods_model
                    .find_position(provider.method().to_glib()),
            );
            self_.image.set_provider(Some(&provider));
            self_
                .title
                .set_text(&i18n::i18n_f("Editing Provider: {}", &[&provider.name()]));
            self_.selected_provider.replace(Some(provider));
        } else {
            self_.name_entry.set_text("");
            self_
                .period_spinbutton
                .set_value(otp::TOTP_DEFAULT_PERIOD as f64);
            self_.provider_website_entry.set_text("");
            self_.provider_help_entry.set_text("");

            self_.algorithm_comborow.set_selected(
                self_
                    .algorithms_model
                    .find_position(Algorithm::default().to_glib()),
            );

            self_
                .default_counter_spinbutton
                .set_value(otp::HOTP_DEFAULT_COUNTER as f64);
            self_
                .digits_spinbutton
                .set_value(otp::DEFAULT_DIGITS as f64);

            self_.method_comborow.set_selected(
                self_
                    .methods_model
                    .find_position(OTPMethod::default().to_glib()),
            );
            self_.image.set_provider(None);
            self_.title.set_text(&gettext("New Provider"));
            self_.selected_provider.replace(None);
        }
    }

    // Validate the information typed by the user in order to enable/disable the save action
    // Note that we don't validate the urls other than: does `url` crate can parse it or not
    fn validate(&self) {
        let self_ = imp::ProviderPage::from_instance(self);

        let provider_name = self_.name_entry.get_text();
        let provider_website = self_.provider_website_entry.get_text();
        let provider_help_url = self_.provider_help_entry.get_text();

        let is_valid = !provider_name.is_empty()
            && (provider_website.is_empty() || url::Url::parse(&provider_website).is_ok())
            && (provider_help_url.is_empty() || url::Url::parse(&provider_help_url).is_ok());

        get_action!(self_.actions, @save).set_enabled(is_valid);
    }

    // Save the provider & emit a signal when one is created/updated
    fn save(&self) -> anyhow::Result<()> {
        let self_ = imp::ProviderPage::from_instance(self);

        let name = self_.name_entry.get_text();
        let website = self_.provider_website_entry.get_text().to_string();
        let help_url = self_.provider_help_entry.get_text().to_string();
        let period = self_.period_spinbutton.get_value() as u32;
        let digits = self_.digits_spinbutton.get_value() as u32;
        let method = OTPMethod::from(self_.method_comborow.get_selected());
        let algorithm = Algorithm::from(self_.algorithm_comborow.get_selected());
        let default_counter = self_.default_counter_spinbutton.get_value() as u32;

        if let Some(provider) = self_.selected_provider.borrow().clone() {
            println!("updating provider");
        } else {
            let provider = Provider::create(
                &name,
                period,
                algorithm,
                Some(website),
                method,
                digits,
                default_counter,
                Some(help_url),
            )?;
            self.emit("created", &[&provider]).unwrap();
        }
        Ok(())
    }

    fn setup_actions(&self) {
        let self_ = imp::ProviderPage::from_instance(self);
        action!(
            self_.actions,
            "save",
            clone!(@weak self as page => move |_, _| {
                if let Err(err) = page.save() {
                    warn!("Failed to save provider {}", err);
                }
            })
        );
        self.insert_action_group("providers", Some(&self_.actions));
        get_action!(self_.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderPage::from_instance(self);
        self_
            .algorithm_comborow
            .set_model(Some(&self_.algorithms_model));

        self_.method_comborow.connect_property_selected_item_notify(
            clone!(@weak self as page => move |_| {
                page.on_method_changed();
            }),
        );

        let validate_cb = clone!(@weak self as page => move |_: &gtk::Entry| {
            page.validate();
        });

        self_.name_entry.connect_changed(validate_cb.clone());
        self_
            .provider_website_entry
            .connect_changed(validate_cb.clone());
        self_.provider_help_entry.connect_changed(validate_cb);

        self_.method_comborow.set_model(Some(&self_.methods_model));
    }

    fn on_method_changed(&self) {
        let self_ = imp::ProviderPage::from_instance(self);

        let selected = OTPMethod::from(self_.method_comborow.get_selected());
        match selected {
            OTPMethod::TOTP => {
                self_.default_counter_row.hide();
                self_.period_row.show();
                self_
                    .digits_spinbutton
                    .set_value(otp::DEFAULT_DIGITS as f64);
                self_
                    .period_spinbutton
                    .set_value(otp::TOTP_DEFAULT_PERIOD as f64);
            }
            OTPMethod::HOTP => {
                self_.default_counter_row.show();
                self_.period_row.hide();
                self_
                    .default_counter_spinbutton
                    .set_value(otp::HOTP_DEFAULT_COUNTER as f64);
                self_
                    .digits_spinbutton
                    .set_value(otp::DEFAULT_DIGITS as f64);
            }
            OTPMethod::Steam => {
                self_.default_counter_row.hide();
                self_.period_row.show();
                self_
                    .digits_spinbutton
                    .set_value(otp::STEAM_DEFAULT_DIGITS as f64);
                self_
                    .period_spinbutton
                    .set_value(otp::STEAM_DEFAULT_PERIOD as f64);
                self_
                    .algorithm_comborow
                    .set_selected(Algorithm::default().to_glib() as u32);
            }
        }

        self_
            .algorithm_comborow
            .set_sensitive(selected != OTPMethod::Steam);
        self_.period_row.set_sensitive(selected != OTPMethod::Steam);
        self_.digits_row.set_sensitive(selected != OTPMethod::Steam);
    }
}
