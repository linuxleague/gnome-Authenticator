use std::str::FromStr;

use anyhow::Result;
use gettextrs::gettext;
use glib::{clone, signal::Inhibit};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::spawn;
use once_cell::sync::{Lazy, OnceCell};

use crate::{
    models::{otp, Account, OTPMethod, OTPUri, Provider, ProvidersModel},
    widgets::{providers::ProviderPage, Camera, ErrorRevealer, ProviderImage, UrlRow},
};

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::subclass::{InitializingObject, Signal};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_add.ui")]
    pub struct AccountAddDialog {
        pub model: OnceCell<ProvidersModel>,
        pub selected_provider: RefCell<Option<Provider>>,
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
        #[template_child]
        pub provider_page: TemplateChild<ProviderPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountAddDialog {
        const NAME: &'static str = "AccountAddDialog";
        type Type = super::AccountAddDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("add.previous", None, move |dialog, _, _| {
                let imp = dialog.imp();
                if imp.deck.visible_child_name().unwrap() != "main" {
                    imp.deck.set_visible_child_name("main");
                } else {
                    dialog.close();
                }
            });

            klass.install_action("add.save", None, move |dialog, _, _| {
                if dialog.save().is_ok() {
                    dialog.close();
                }
            });

            klass.install_action("add.camera", None, move |dialog, _, _| {
                dialog.scan_from_camera();
            });

            klass.install_action("add.screenshot", None, move |dialog, _, _| {
                dialog.scan_from_screenshot();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountAddDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("added", &[], <()>::static_type().into())
                    .action()
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.action_set_enabled("add.save", false);
        }
    }
    impl WidgetImpl for AccountAddDialog {}
    impl WindowImpl for AccountAddDialog {}
    impl AdwWindowImpl for AccountAddDialog {}
}
glib::wrapper! {
    pub struct AccountAddDialog(ObjectSubclass<imp::AccountAddDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

#[gtk::template_callbacks]
impl AccountAddDialog {
    pub fn new(model: ProvidersModel) -> Self {
        let dialog = glib::Object::new::<Self>(&[]).expect("Failed to create AccountAddDialog");

        dialog.imp().model.set(model).unwrap();
        dialog.setup_widget();
        dialog
    }

    pub fn connect_added<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_local(
            "added",
            false,
            clone!(@weak self as dialog => @default-return None, move |_| {
                callback(&dialog);
                None
            }),
        )
    }

    #[template_callback]
    fn input_validate(&self, _: Option<gtk::Editable>) {
        let imp = self.imp();
        let username = imp.username_entry.text();
        let token = imp.token_entry.text();
        let has_provider = imp.selected_provider.borrow().is_some();

        let is_valid = !username.is_empty() && !token.is_empty() && has_provider;
        self.action_set_enabled("add.save", is_valid);
    }

    #[template_callback]
    fn match_selected(&self, store: gtk::ListStore, iter: gtk::TreeIter) -> Inhibit {
        let provider_id = store.get::<u32>(&iter, 0);
        let provider = self.imp().model.get().unwrap().find_by_id(provider_id);
        self.set_provider(provider);

        Inhibit(false)
    }

    #[template_callback]
    fn no_matches_selected(&self, completion: gtk::EntryCompletion) {
        // in case the provider doesn't exists, let the user create a new one by showing
        // a dialog for that TODO: replace this whole completion provider thing
        // with a custom widget
        let imp = self.imp();
        let entry = completion.entry().unwrap();

        imp.deck.set_visible_child_name("create-provider");
        imp.provider_page
            .imp()
            .back_btn
            .set_action_name(Some("add.previous"));
        imp.provider_page.imp().revealer.set_reveal_child(true);
        imp.provider_page.set_provider(None);

        let name_entry = imp.provider_page.name_entry();
        name_entry.set_text(&entry.text());
        name_entry.grab_focus_without_selecting();
        name_entry.set_position(entry.cursor_position());
    }

    #[template_callback]
    fn deck_visible_child_name_notify(&self, _pspec: glib::ParamSpec, deck: adw::Leaflet) {
        if deck.visible_child_name().as_deref() != Some("camera") {
            self.imp().camera.stop();
        }
    }

    #[template_callback]
    fn camera_closed(&self, _camera: Camera) {
        self.activate_action("add.previous", None).unwrap();
    }

    #[template_callback]
    fn camera_code_detected(&self, _camera: Camera, code: String) {
        if let Ok(otp_uri) = OTPUri::from_str(&code) {
            self.set_from_otp_uri(&otp_uri);
        }
    }

    #[template_callback]
    fn provider_created(&self, provider: Provider, _page: ProviderPage) {
        let imp = self.imp();
        let model = imp.model.get().unwrap();
        model.append(&provider);

        imp.provider_completion
            .set_model(Some(&model.completion_model()));
        self.set_provider(Some(provider));
        imp.username_entry.grab_focus_without_selecting();
        imp.deck.set_visible_child_name("main");
    }

    fn scan_from_screenshot(&self) {
        spawn!(clone!(@weak self as page => async move {
            if let Err(err) = page.imp().camera.scan_from_screenshot().await {
                tracing::error!("Failed to scan from screenshot {}", err);
            }
        }));
    }

    fn scan_from_camera(&self) {
        let imp = self.imp();
        imp.camera.scan_from_camera();
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

            let account = Account::create(&username, &token, None, provider)?;

            imp.model.get().unwrap().add_account(&account, provider);
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
        self.input_validate(None);
    }

    fn setup_widget(&self) {
        let imp = self.imp();
        imp.provider_completion
            .set_model(Some(&imp.model.get().unwrap().completion_model()));
    }
}
