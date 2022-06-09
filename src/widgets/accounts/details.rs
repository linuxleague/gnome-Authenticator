use super::qrcode_paintable::QRCodePaintable;
use crate::{
    models::{Account, OTPMethod, Provider, ProvidersModel},
    widgets::UrlRow,
};
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
mod imp {
    use crate::{
        models::Provider,
        widgets::{editable_label::EditableSpin, EditableLabel},
    };

    use super::*;
    use glib::subclass::{self, Signal};
    use once_cell::sync::{Lazy, OnceCell};
    use std::cell::RefCell;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/account_details_page.ui")]
    pub struct AccountDetailsPage {
        #[template_child]
        pub website_row: TemplateChild<UrlRow>,
        #[template_child]
        pub qrcode_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub provider_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub account_label: TemplateChild<EditableLabel>,
        #[template_child(id = "list")]
        pub listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub algorithm_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub method_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub counter_label: TemplateChild<EditableSpin>,
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
        #[template_child]
        pub edit_stack: TemplateChild<gtk::Stack>,
        pub qrcode_paintable: QRCodePaintable,
        pub account: RefCell<Option<Account>>,
        #[template_child]
        pub provider_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub provider_completion: TemplateChild<gtk::EntryCompletion>,
        #[template_child]
        pub provider_entry: TemplateChild<gtk::Entry>,
        pub selected_provider: RefCell<Option<Provider>>,
        pub providers_model: OnceCell<ProvidersModel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountDetailsPage {
        const NAME: &'static str = "AccountDetailsPage";
        type Type = super::AccountDetailsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            EditableLabel::static_type();
            EditableSpin::static_type();

            klass.install_action("account.delete", None, move |page, _, _| {
                page.delete_account();
            });
            klass.install_action("account.edit", None, move |page, _, _| {
                page.set_edit_mode();
            });
            klass.install_action("account.save", None, move |page, _, _| {
                if let Err(err) = page.save() {
                    tracing::error!("Failed to save account details {}", err);
                }
            });

            klass.install_action("account.back", None, move |page, _, _| {
                let imp = page.imp();
                if imp.edit_stack.visible_child_name().as_deref() == Some("save") {
                    imp.edit_stack.set_visible_child_name("edit");
                    imp.account_label.stop_editing(false);
                    imp.counter_label.stop_editing(false);
                    imp.provider_stack.set_visible_child_name("display");
                    imp.edit_stack.grab_focus();
                } else {
                    page.activate_action("win.back", None).unwrap();
                }
            });

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "account.back",
                None,
            );
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountDetailsPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder(
                        "removed",
                        &[Account::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .action()
                    .build(),
                    Signal::builder("provider-changed", &[], <()>::static_type().into())
                        .action()
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.init_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountDetailsPage {
        fn unmap(&self, widget: &Self::Type) {
            self.edit_stack.set_visible_child_name("edit");
            self.account_label.stop_editing(false);
            self.counter_label.stop_editing(false);
            self.provider_stack.set_visible_child_name("display");
            self.provider_entry.set_text("");
            self.parent_unmap(widget);
        }
    }
    impl BoxImpl for AccountDetailsPage {}
}

glib::wrapper! {
    pub struct AccountDetailsPage(ObjectSubclass<imp::AccountDetailsPage>) @extends gtk::Widget, gtk::Box;
}

impl AccountDetailsPage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create AccountDetailsPage")
    }

    fn init_widgets(&self) {
        let imp = self.imp();
        imp.qrcode_picture
            .set_paintable(Some(&imp.qrcode_paintable));
        imp.counter_label.set_adjustment(1, u32::MAX);

        imp.provider_completion.connect_match_selected(
            clone!(@weak self as page  => @default-return gtk::Inhibit(false), move |_, store, iter| {
                let provider_id = store.get::<u32>(iter, 0);
                let model = page.imp().providers_model.get().unwrap();
                let provider = model.find_by_id(provider_id);
                page.set_provider(provider.unwrap_or_else(|| {
                    page.imp().account.borrow().as_ref().unwrap().provider()
                }));

                gtk::Inhibit(false)
            }),
        );
    }

    fn delete_account(&self) {
        let parent = self.root().unwrap().downcast::<gtk::Window>().unwrap();

        let dialog = gtk::MessageDialog::builder()
            .message_type(gtk::MessageType::Warning)
            .buttons(gtk::ButtonsType::YesNo)
            .text(&gettext("Are you sure you want to delete the account?"))
            .secondary_text(&gettext("This action is irreversible"))
            .modal(true)
            .transient_for(&parent)
            .build();
        dialog.connect_response(clone!(@weak self as page => move |dialog, response| {
            if response == gtk::ResponseType::Yes {
                let account = page.imp().account.borrow().as_ref().unwrap().clone();
                page.emit_by_name::<()>("removed", &[&account]);
            }
            dialog.close();
        }));

        dialog.show();
    }

    pub fn set_account(&self, account: &Account) {
        let imp = self.imp();
        let qr_code = account.qr_code();
        imp.qrcode_paintable.set_qrcode(qr_code);

        if account.provider().method() == OTPMethod::HOTP {
            imp.counter_label.set_text(account.counter());
        }
        self.set_provider(account.provider());
        imp.account_label.set_text(&account.name());
        imp.account.replace(Some(account.clone()));
    }

    pub fn set_providers_model(&self, model: ProvidersModel) {
        self.imp()
            .provider_completion
            .set_model(Some(&model.completion_model()));
        self.imp().providers_model.set(model).unwrap();
    }

    fn set_provider(&self, provider: Provider) {
        let imp = self.imp();
        imp.provider_label.set_text(&provider.name());
        imp.algorithm_label
            .set_text(&provider.algorithm().to_locale_string());
        imp.method_label
            .set_text(&provider.method().to_locale_string());
        if provider.method() == OTPMethod::HOTP {
            imp.counter_row.show();
            imp.period_row.hide();
        } else {
            imp.counter_row.hide();
            imp.period_row.show();
            imp.period_label.set_text(&provider.period().to_string());
        }
        imp.digits_label.set_text(&provider.digits().to_string());
        if let Some(ref help) = provider.help_url() {
            imp.help_row.set_uri(help);
            imp.help_row.show();
        } else {
            imp.help_row.hide();
        }
        if let Some(ref website) = provider.website() {
            imp.website_row.set_uri(website);
            imp.website_row.show();
        } else {
            imp.website_row.hide();
        }
        imp.selected_provider.replace(Some(provider));
    }

    fn set_edit_mode(&self) {
        let imp = self.imp();
        imp.edit_stack.set_visible_child_name("save");
        imp.account_label.start_editing();
        imp.counter_label.start_editing();
        imp.provider_stack.set_visible_child_name("edit");
        if let Some(account) = imp.account.borrow().as_ref() {
            imp.provider_entry.set_text(&account.provider().name());
        }
        imp.account_label.grab_focus();
    }

    fn save(&self) -> anyhow::Result<()> {
        let imp = self.imp();
        imp.edit_stack.set_visible_child_name("edit");
        imp.account_label.stop_editing(true);
        imp.counter_label.stop_editing(true);
        imp.provider_stack.set_visible_child_name("display");

        if let Some(account) = imp.account.borrow().as_ref() {
            account.set_name(&imp.account_label.text())?;

            if let Some(selected_provider) = imp.selected_provider.borrow().as_ref() {
                let current_provider = account.provider();
                if selected_provider.id() != current_provider.id() {
                    selected_provider.add_account(account);
                    current_provider.remove_account(account);
                    account.set_provider(selected_provider)?;
                    imp.provider_entry.set_text(&selected_provider.name());
                    self.emit_by_name::<()>("provider-changed", &[]);
                }
            }

            let old_counter = account.counter();
            account.set_counter(imp.counter_label.value())?;
            // regenerate the otp value if the counter value was changed
            if old_counter != account.counter() && account.provider().method() == OTPMethod::HOTP {
                account.generate_otp();
            }
        }
        Ok(())
    }
}
