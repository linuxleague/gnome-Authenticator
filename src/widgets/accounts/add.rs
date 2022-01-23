use crate::{
    models::{otp, Account, OTPMethod, OTPUri, Provider, ProvidersModel},
    widgets::{Camera, ErrorRevealer, ProviderImage, UrlRow},
};
use anyhow::Result;
use gettextrs::gettext;
use glib::{clone, signal::Inhibit};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action, spawn};
use once_cell::sync::OnceCell;
use std::str::FromStr;

mod imp {
    use super::*;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, Default, CompositeTemplate)]
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
        #[template_child]
        pub error_revealer: TemplateChild<ErrorRevealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountAddDialog {
        const NAME: &'static str = "AccountAddDialog";
        type Type = super::AccountAddDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountAddDialog {
        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("added", &[], <()>::static_type().into())
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
        let dialog = glib::Object::new::<Self>(&[]).expect("Failed to create AccountAddDialog");

        dialog.imp().model.set(model).unwrap();
        dialog.setup_actions();
        dialog.setup_signals();
        dialog.setup_widgets();
        dialog
    }

    fn validate(&self) {
        let imp = self.imp();
        let username = imp.username_entry.text();
        let token = imp.token_entry.text();

        let is_valid = !(username.is_empty() || token.is_empty());
        get_action!(imp.actions, @save).set_enabled(is_valid);
    }

    fn setup_signals(&self) {
        let imp = self.imp();

        imp.username_entry
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));
        imp.token_entry
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));
    }

    fn scan_from_screenshot(&self) {
        spawn!(clone!(@weak self as page => async move {
           if let Err(err) = page.imp().camera.from_screenshot().await {
                log::error!("Failed to scan from screenshot {}", err);
           }
        }));
    }

    fn scan_from_camera(&self) {
        let imp = self.imp();
        imp.camera.from_camera();
        imp.deck.set_visible_child_name("camera");
    }

    pub fn set_from_otp_uri(&self, otp_uri: &OTPUri) {
        let imp = self.imp();
        imp.deck.set_visible_child_name("main"); // Switch back the form view

        imp.token_entry.set_text(&otp_uri.secret);
        imp.username_entry.set_text(&otp_uri.label);

        let provider = imp
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
                None,
                None,
            )
            .ok();

        self.set_provider(provider);
    }

    fn save(&self) -> Result<()> {
        let imp = self.imp();

        if let Some(ref provider) = *imp.selected_provider.borrow() {
            let username = imp.username_entry.text();
            let token = imp.token_entry.text();
            if !otp::is_valid(&token) {
                imp.error_revealer.popup(&gettext("Invalid Token"));
                anyhow::bail!("Token {} is not a valid Base32 secret", &token);
            }

            let account = Account::create(&username, &token, provider)?;

            imp.model.get().unwrap().add_account(&account, &provider);
            self.emit_by_name::<()>("added", &[]);
        // TODO: display an error message saying there was an error form keyring
        } else {
            anyhow::bail!("Could not find provider");
        }
        Ok(())
    }

    fn set_provider(&self, provider: Option<Provider>) {
        let imp = self.imp();
        if let Some(provider) = provider {
            imp.more_list.show();
            imp.provider_entry.set_text(&provider.name());
            imp.period_label.set_text(&provider.period().to_string());

            imp.image.set_provider(Some(&provider));

            imp.method_label
                .set_text(&provider.method().to_locale_string());

            imp.algorithm_label
                .set_text(&provider.algorithm().to_locale_string());

            imp.digits_label.set_text(&provider.digits().to_string());

            match provider.method() {
                OTPMethod::TOTP | OTPMethod::Steam => {
                    imp.counter_row.hide();
                    imp.period_row.show();
                }
                OTPMethod::HOTP => {
                    imp.counter_row.show();
                    imp.period_row.hide();
                }
            };

            if let Some(ref website) = provider.website() {
                imp.provider_website_row.set_uri(website);
            }
            if let Some(ref help_url) = provider.help_url() {
                imp.provider_help_row.set_uri(help_url);
            }
            imp.selected_provider.borrow_mut().replace(provider);
        } else {
            imp.selected_provider.borrow_mut().take();
        }
    }

    fn setup_actions(&self) {
        let imp = self.imp();
        action!(
            imp.actions,
            "previous",
            clone!(@weak self as dialog => move |_, _| {
                let imp = dialog.imp();
                if imp.deck.visible_child_name().unwrap() == "camera" {
                    imp.deck.set_visible_child_name("main");
                } else {
                    dialog.close();
                }
            })
        );

        action!(
            imp.actions,
            "save",
            clone!(@weak self as dialog => move |_, _| {
                if dialog.save().is_ok() {
                    dialog.close();
                }
            })
        );

        action!(
            imp.actions,
            "camera",
            clone!(@weak self as dialog => move |_, _| {
                dialog.scan_from_camera();
            })
        );

        action!(
            imp.actions,
            "screenshot",
            clone!(@weak self as dialog => move |_, _| {
                dialog.scan_from_screenshot();
            })
        );
        self.insert_action_group("add", Some(&imp.actions));
        get_action!(imp.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        imp.provider_completion
            .set_model(Some(&imp.model.get().unwrap().completion_model()));

        imp.provider_completion.connect_match_selected(
            clone!(@weak self as dialog, @strong imp.model as model => @default-return Inhibit(false), move |_, store, iter| {
                let provider_id = store.get::<u32>(iter, 0);
                let provider = model.get().unwrap().find_by_id(provider_id);
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );

        imp.deck
            .connect_visible_child_name_notify(clone!(@weak self as page => move |deck| {
                if deck.visible_child_name().as_ref().map(|s|s.as_str()) != Some("camera") {
                    page.imp().camera.stop();
                }
            }));

        imp.camera.connect_local(
            "code-detected",
            false,
            clone!(@weak self as dialog => @default-return None, move |args| {
                let code = args.get(1).unwrap().get::<String>().unwrap();
                if let Ok(otp_uri) = OTPUri::from_str(&code) {
                    dialog.set_from_otp_uri(&otp_uri);
                }

                None
            }),
        );
    }
}
