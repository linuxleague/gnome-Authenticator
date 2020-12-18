use crate::{
    helpers::qrcode,
    models::{Account, OTPMethod, OTPUri, Provider, ProvidersModel},
    widgets::{ProviderImage, UrlRow},
};
use anyhow::Result;
use gio::{subclass::ObjectSubclass, ActionMapExt};
use glib::{clone, signal::Inhibit};
use gtk::{gio, glib, prelude::*, CompositeTemplate};
use gtk_macros::{action, get_action};
use once_cell::sync::OnceCell;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(CompositeTemplate)]
    pub struct AccountAddDialog {
        pub model: OnceCell<ProvidersModel>,
        pub selected_provider: RefCell<Option<Provider>>,
        pub actions: gio::SimpleActionGroup,
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
        pub counter_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub period_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub provider_completion: TemplateChild<gtk::EntryCompletion>,
    }

    impl ObjectSubclass for AccountAddDialog {
        const NAME: &'static str = "AccountAddDialog";
        type Type = super::AccountAddDialog;
        type ParentType = libhandy::Window;
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
            }
        }

        fn class_init(klass: &mut Self::Class) {
            UrlRow::static_type();
            ProviderImage::static_type();
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/account_add.ui");
            Self::bind_template_children(klass);
            klass.add_signal("added", glib::SignalFlags::ACTION, &[], glib::Type::Unit);
        }
    }

    impl ObjectImpl for AccountAddDialog {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountAddDialog {}
    impl WindowImpl for AccountAddDialog {}
    impl libhandy::subclass::window::WindowImpl for AccountAddDialog {}
}
glib::wrapper! {
    pub struct AccountAddDialog(ObjectSubclass<imp::AccountAddDialog>) @extends gtk::Widget, gtk::Window, libhandy::Window;
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
        let username = self_.username_entry.get().get_text().unwrap();
        let token = self_.token_entry.get().get_text().unwrap();

        let is_valid = !(username.is_empty() || token.is_empty());
        get_action!(self_.actions, @save).set_enabled(is_valid);
    }

    fn setup_signals(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);

        self_
            .username_entry
            .get()
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));
        self_
            .token_entry
            .get()
            .connect_changed(clone!(@weak self as win => move |_| win.validate()));

        let event_controller = gtk::EventControllerKey::new();
        event_controller.connect_key_pressed(
            clone!(@weak self as widget => @default-return Inhibit(false), move |_, k, _, _| {
                if k == 65307 {
                    widget.close();
                }
                Inhibit(false)
            }),
        );
        self.add_controller(&event_controller);
    }

    fn scan_qr(&self) -> Result<()> {
        qrcode::screenshot_area(
            self.clone().upcast::<gtk::Window>(),
            clone!(@weak self as dialog => move |screenshot| {
                if let Ok(otp_uri) = qrcode::scan(&screenshot) {
                    dialog.set_from_otp_uri(otp_uri);
                }
            }),
        )?;
        Ok(())
    }

    fn set_from_otp_uri(&self, otp_uri: OTPUri) {
        let self_ = imp::AccountAddDialog::from_instance(self);

        self_.token_entry.get().set_text(&otp_uri.secret);
        self_.username_entry.get().set_text(&otp_uri.label);

        let provider = self_
            .model
            .get()
            .unwrap()
            .find_or_create(
                &otp_uri.issuer,
                otp_uri.period.unwrap_or(30),
                otp_uri.method,
                None,
                otp_uri.algorithm,
                otp_uri.digits.unwrap_or(6),
                otp_uri.counter.unwrap_or(1),
            )
            .unwrap();

        self.set_provider(provider);
    }

    fn save(&self) -> Result<()> {
        let self_ = imp::AccountAddDialog::from_instance(self);

        if let Some(ref provider) = *self_.selected_provider.borrow() {
            let username = self_.username_entry.get().get_text().unwrap();
            let token = self_.token_entry.get().get_text().unwrap();

            let account = Account::create(&username, &token, provider)?;

            self_.model.get().unwrap().add_account(&account, &provider);
            self.emit("added", &[]).unwrap();
            // TODO: display an error message saying there was an error form keyring
        }
        Ok(())
    }

    fn set_provider(&self, provider: Provider) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_.more_list.get().show();
        self_.provider_entry.get().set_text(&provider.name());
        self_
            .period_label
            .get()
            .set_text(&provider.period().to_string());

        self_.image.get().set_provider(&provider);

        self_
            .method_label
            .get()
            .set_text(&provider.method().to_locale_string());

        self_
            .algorithm_label
            .get()
            .set_text(&provider.algorithm().to_locale_string());

        self_
            .digits_label
            .get()
            .set_text(&provider.digits().to_string());

        match provider.method() {
            OTPMethod::TOTP => {
                self_.counter_row.get().hide();
                self_.period_row.get().show();
            }
            OTPMethod::HOTP => {
                self_.counter_row.get().show();
                self_.period_row.get().hide();
            }
            OTPMethod::Steam => {}
        };

        if let Some(ref website) = provider.website() {
            self_.provider_website_row.get().set_uri(website);
        }
        if let Some(ref help_url) = provider.help_url() {
            self_.provider_help_row.get().set_uri(help_url);
        }
        self_.selected_provider.borrow_mut().replace(provider);
    }

    fn setup_actions(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        action!(
            self_.actions,
            "back",
            clone!(@weak self as dialog => move |_, _| {
                dialog.destroy();
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
            "scan-qr",
            clone!(@weak self as dialog => move |_, _| {
                dialog.scan_qr();
            })
        );
        self.insert_action_group("add", Some(&self_.actions));
        get_action!(self_.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        self_
            .provider_completion
            .get()
            .set_model(Some(&self_.model.get().unwrap().completion_model()));

        self_.provider_completion.get().connect_match_selected(
            clone!(@weak self as dialog, @strong self_.model as model => move |_, store, iter| {
                let provider_id = store.get_value(iter, 0). get_some::<i32>().unwrap();
                let provider = model.get().unwrap().find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );
    }
}
