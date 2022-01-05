use super::qrcode_paintable::QRCodePaintable;
use crate::{models::Account, widgets::UrlRow};
use gettextrs::gettext;
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
mod imp {
    use super::*;
    use glib::subclass::{self, Signal};
    use once_cell::sync::Lazy;
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
        pub account_label: TemplateChild<gtk::Label>,
        #[template_child(id = "list")]
        pub listbox: TemplateChild<gtk::ListBox>,
        pub qrcode_paintable: QRCodePaintable,
        pub account: RefCell<Option<Account>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountDetailsPage {
        const NAME: &'static str = "AccountDetailsPage";
        type Type = super::AccountDetailsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("account.delete", None, move |page, _, _| {
                page.delete_account();
            });
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountDetailsPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "removed",
                    &[Account::static_type().into()],
                    <()>::static_type().into(),
                )
                .flags(glib::SignalFlags::ACTION)
                .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.init_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountDetailsPage {}
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

        imp.account_label.set_text(&account.name());
        imp.provider_label.set_text(&account.provider().name());

        if let Some(ref website) = account.provider().website() {
            imp.website_row.set_uri(website);
            imp.website_row.show();
        } else {
            imp.website_row.hide();
        }
        imp.account.replace(Some(account.clone()));
    }
}
