use crate::{
    models::{Account, OTPMethod, OTPUri, Provider, ProvidersModel},
    widgets::{Camera, ProviderImage, UrlRow},
};
use anyhow::Result;
use glib::{clone, signal::Inhibit};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;
use std::str::FromStr;

mod imp {
    use super::*;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_add.ui")]
    pub struct AccountAddDialog {
        pub model: OnceCell<ProvidersModel>,
        pub selected_provider: RefCell<Option<Provider>>,
        pub actions: gio::SimpleActionGroup,
        #[template_child]
        pub camera: TemplateChild<Camera>,
        #[template_child]
        pub deck: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub image: TemplateChild<ProviderImage>,
        #[template_child]
        pub provider_website_row: TemplateChild<UrlRow>,
        #[template_child]
        pub provider_help_row: TemplateChild<UrlRow>,
        #[template_child]
        pub username_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub token_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub more_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub period_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub digits_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub provider_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub method_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub algorithm_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub counter_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub period_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub provider_completion: TemplateChild<gtk::EntryCompletion>,
    }

    impl ObjectSubclass for AccountAddDialog {
        const NAME: &'static str = "AccountAddDialog";
        type Type = super::AccountAddDialog;
        type ParentType = adw::Window;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let actions = gio::SimpleActionGroup::new();

            Self {
                actions,
                model: OnceCell::new(),
                selected_provider: RefCell::new(None),
                image: TemplateChild::default(),
                provider_website_row: TemplateChild::default(),
                provider_help_row: TemplateChild::default(),
                token_entry: TemplateChild::default(),
                username_entry: TemplateChild::default(),
                more_list: TemplateChild::default(),
                period_label: TemplateChild::default(),
                digits_label: TemplateChild::default(),
                provider_entry: TemplateChild::default(),
                method_label: TemplateChild::default(),
                provider_completion: TemplateChild::default(),
                algorithm_label: TemplateChild::default(),
                counter_row: TemplateChild::default(),
                period_row: TemplateChild::default(),
                deck: TemplateChild::default(),
                camera: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountAddDialog {
        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("added", &[], <()>::static_type())
                    .flags(glib::SignalFlags::ACTION)
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for AccountAddDialog {}
    impl WindowImpl for AccountAddDialog {}
    impl adw::subclass::window::AdwWindowImpl for AccountAddDialog {}
}
glib::wrapper! {
    pub struct AccountAddDialog(ObjectSubclass<imp::AccountAddDialog>) @extends gtk::Widget, gtk::Window, adw::Window;
}

impl AccountAddDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog = glib::Object::new(&[]).expect("Failed to create AccountAddDialog");

        let self_ = imp::AccountAddDialog::from_instance(&dialog);
        self_.model.set(model).unwrap();

        dialog.setup_actions();
        dialog.setup_signals();
        dialog.setup_widgets();
        dialog
    }

    fn validate(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        let username = self_.username_entry.get_text().unwrap();
        let token = self_.token_entry.get_text().unwrap();

        let is_valid = !(username.is_empty() || token.is_empty());
        get_action!(self_.actions, @save).set_enabled(is_valid);
    }

    fn setup_signals(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);

        self_
            .username_entry
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));
        self_
            .token_entry
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));
    }

    fn scan_from_screenshot(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_.camera.from_screenshot();
    }

    fn scan_from_camera(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_.deck.set_visible_child_name("camera");

        self_.camera.start();
    }

    fn set_from_otp_uri(&self, otp_uri: OTPUri) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_.deck.set_visible_child_name("main"); // Switch back the form view

        self_.token_entry.set_text(&otp_uri.secret);
        self_.username_entry.set_text(&otp_uri.label);

        let provider = self_
            .model
            .get()
            .unwrap()
            .find_or_create(
                &otp_uri.issuer,
                otp_uri.period,
                otp_uri.method,
                None,
                otp_uri.algorithm,
                otp_uri.digits,
                otp_uri.counter,
            )
            .unwrap();

        self.set_provider(provider);
    }

    fn save(&self) -> Result<()> {
        let self_ = imp::AccountAddDialog::from_instance(self);

        if let Some(ref provider) = *self_.selected_provider.borrow() {
            let username = self_.username_entry.get_text().unwrap();
            let token = self_.token_entry.get_text().unwrap();

            let account = Account::create(&username, &token, provider)?;

            self_.model.get().unwrap().add_account(&account, &provider);
            self.emit("added", &[]).unwrap();
            // TODO: display an error message saying there was an error form keyring
        }
        Ok(())
    }

    fn set_provider(&self, provider: Provider) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_.more_list.show();
        self_.provider_entry.set_text(&provider.name());
        self_.period_label.set_text(&provider.period().to_string());

        self_.image.set_provider(&provider);

        self_
            .method_label
            .set_text(&provider.method().to_locale_string());

        self_
            .algorithm_label
            .set_text(&provider.algorithm().to_locale_string());

        self_.digits_label.set_text(&provider.digits().to_string());

        match provider.method() {
            OTPMethod::TOTP => {
                self_.counter_row.hide();
                self_.period_row.show();
            }
            OTPMethod::HOTP => {
                self_.counter_row.show();
                self_.period_row.hide();
            }
            OTPMethod::Steam => {}
        };

        if let Some(ref website) = provider.website() {
            self_.provider_website_row.set_uri(website);
        }
        if let Some(ref help_url) = provider.help_url() {
            self_.provider_help_row.set_uri(help_url);
        }
        self_.selected_provider.borrow_mut().replace(provider);
    }

    fn setup_actions(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        action!(
            self_.actions,
            "back",
            clone!(@weak self as dialog => move |_, _| {
                dialog.close();
            })
        );
        action!(
            self_.actions,
            "save",
            clone!(@weak self as dialog => move |_, _| {
                if dialog.save().is_ok() {
                    dialog.close();
                }
            })
        );

        action!(
            self_.actions,
            "camera",
            clone!(@weak self as dialog => move |_, _| {
                dialog.scan_from_camera();
            })
        );

        action!(
            self_.actions,
            "screenshot",
            clone!(@weak self as dialog => move |_, _| {
                dialog.scan_from_screenshot();
            })
        );
        self.insert_action_group("add", Some(&self_.actions));
        get_action!(self_.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_
            .provider_completion
            .set_model(Some(&self_.model.get().unwrap().completion_model()));

        self_.provider_completion.connect_match_selected(
            clone!(@weak self as dialog, @strong self_.model as model => move |_, store, iter| {
                let provider_id = store.get_value(iter, 0).get_some::<u32>().unwrap();
                let provider = model.get().unwrap().find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );

        self_
            .camera
            .connect_local(
                "code-detected",
                false,
                clone!(@weak self as dialog => move |args| {
                    let code = args.get(1).unwrap().get::<String>().unwrap().unwrap();
                    if let Ok(otp_uri) = OTPUri::from_str(&code) {
                        dialog.set_from_otp_uri(otp_uri);
                    }

                    None
                }),
            )
            .unwrap();
    }
}
