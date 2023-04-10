use adw::prelude::*;
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, clone},
    subclass::prelude::*,
};

use super::{QRCodeData, QRCodePaintable};
use crate::{
    models::{Account, Provider, ProvidersModel},
    widgets::{providers::ProviderEntryRow, ProvidersDialog, UrlRow},
};
mod imp {
    use std::cell::RefCell;

    use glib::subclass::Signal;
    use once_cell::sync::{Lazy, OnceCell};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_details_page.ui")]
    #[properties(wrapper_type = super::AccountDetailsPage)]
    pub struct AccountDetailsPage {
        #[template_child]
        pub website_row: TemplateChild<UrlRow>,
        #[template_child]
        pub qrcode_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub account_label: TemplateChild<adw::EntryRow>,
        #[template_child(id = "list")]
        pub listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub algorithm_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub method_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub counter_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub period_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub digits_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub counter_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub period_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub help_row: TemplateChild<UrlRow>,
        pub qrcode_paintable: QRCodePaintable,
        pub account: RefCell<Option<Account>>,
        #[template_child]
        pub provider_entry: TemplateChild<ProviderEntryRow>,
        pub selected_provider: RefCell<Option<Provider>>,
        #[property(get, set)]
        pub model: OnceCell<ProvidersModel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountDetailsPage {
        const NAME: &'static str = "AccountDetailsPage";
        type Type = super::AccountDetailsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action("account.delete", None, move |page, _, _| {
                page.delete_account();
            });
            klass.install_action("account.save", None, move |page, _, _| {
                if let Err(err) = page.save() {
                    tracing::error!("Failed to save account details {}", err);
                }
            });

            klass.install_action("account.back", None, move |page, _, _| {
                page.activate_action("win.back", None).unwrap();
            });

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "account.back",
                None,
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountDetailsPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("removed")
                        .param_types([Account::static_type()])
                        .action()
                        .build(),
                    Signal::builder("provider-changed").action().build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.qrcode_picture
                .set_paintable(Some(&self.qrcode_paintable));
        }
    }
    impl WidgetImpl for AccountDetailsPage {}
    impl BoxImpl for AccountDetailsPage {}
}

glib::wrapper! {
    pub struct AccountDetailsPage(ObjectSubclass<imp::AccountDetailsPage>)
        @extends gtk::Widget, gtk::Box;
}

#[gtk::template_callbacks]
impl AccountDetailsPage {
    fn delete_account(&self) {
        let parent = self.root().and_downcast::<gtk::Window>().unwrap();

        let dialog = adw::MessageDialog::builder()
            .heading(gettext("Are you sure you want to delete the account?"))
            .body(gettext("This action is irreversible"))
            .modal(true)
            .transient_for(&parent)
            .build();
        dialog.add_responses(&[("no", &gettext("No")), ("yes", &gettext("Yes"))]);
        dialog.set_response_appearance("yes", adw::ResponseAppearance::Destructive);
        dialog.connect_response(
            None,
            clone!(@weak self as page => move |dialog, response| {
                if response == "yes" {
                    let account = page.imp().account.borrow().as_ref().unwrap().clone();
                    page.emit_by_name::<()>("removed", &[&account]);
                }
                dialog.close();
            }),
        );

        dialog.present();
    }

    #[template_callback]
    fn on_provider_create(&self, entry: ProviderEntryRow) {
        let model = self.model();
        let window = self.root().and_downcast::<gtk::Window>();
        let dialog = ProvidersDialog::new(&model);
        dialog.create_with(&entry.text());
        dialog.connect_changed(move |_dialog, provider| {
            entry.set_selected_provider(Some(provider), true);
        });
        dialog.set_transient_for(window.as_ref());
        dialog.present();
    }

    #[template_callback]
    fn on_provider_notify(&self) {
        if let Some(provider) = self.imp().provider_entry.provider() {
            self.set_provider(provider);
        }
    }

    pub fn set_account(&self, account: &Account) {
        let imp = self.imp();
        let qr_code = QRCodeData::from(String::from(account.otp_uri()));
        imp.qrcode_paintable.set_qrcode(qr_code);

        if account.provider().method().is_event_based() {
            imp.counter_spinbutton.set_value(account.counter() as f64);
        }
        imp.provider_entry
            .set_selected_provider(Some(account.provider()), true);
        self.set_provider(account.provider());
        imp.account_label.set_text(&account.name());
        imp.account.replace(Some(account.clone()));
    }

    fn set_provider(&self, provider: Provider) {
        let imp = self.imp();
        imp.algorithm_label
            .set_text(&provider.algorithm().to_locale_string());
        imp.method_label
            .set_text(&provider.method().to_locale_string());
        if provider.method().is_event_based() {
            imp.counter_row.set_visible(true);
            imp.period_row.set_visible(false);
        } else {
            imp.counter_row.set_visible(false);
            imp.period_row.set_visible(true);
            imp.period_label.set_text(&provider.period().to_string());
        }
        imp.digits_label.set_text(&provider.digits().to_string());
        if let Some(help) = provider.help_url() {
            imp.help_row.set_uri(help);
            imp.help_row.set_visible(true);
        } else {
            imp.help_row.set_visible(false);
        }
        if let Some(website) = provider.website() {
            imp.website_row.set_uri(website);
            imp.website_row.set_visible(true);
        } else {
            imp.website_row.set_visible(false);
        }
        imp.selected_provider.replace(Some(provider));
    }

    fn save(&self) -> anyhow::Result<()> {
        let imp = self.imp();

        if let Some(account) = imp.account.borrow().as_ref() {
            account.set_name(imp.account_label.text());

            if let Some(selected_provider) = imp.selected_provider.borrow().as_ref() {
                let current_provider = account.provider();
                if selected_provider.id() != current_provider.id() {
                    selected_provider.add_account(account);
                    current_provider.remove_account(account);
                    account.set_provider(selected_provider)?;
                    self.emit_by_name::<()>("provider-changed", &[]);
                }
            }

            let old_counter = account.counter();
            account.set_counter(imp.counter_spinbutton.value() as u32);
            // regenerate the otp value if the counter value was changed
            if old_counter != account.counter() && account.provider().method().is_event_based() {
                account.generate_otp();
            }
        }
        Ok(())
    }
}
