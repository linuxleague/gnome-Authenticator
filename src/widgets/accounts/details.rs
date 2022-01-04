use super::qrcode_paintable::QRCodePaintable;
use crate::{models::Account, widgets::UrlRow};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass;

    #[derive(Debug, CompositeTemplate)]
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountDetailsPage {
        const NAME: &'static str = "AccountDetailsPage";
        type Type = super::AccountDetailsPage;
        type ParentType = gtk::Box;

        fn new() -> Self {
            Self {
                qrcode_picture: TemplateChild::default(),
                account_label: TemplateChild::default(),
                provider_label: TemplateChild::default(),
                website_row: TemplateChild::default(),
                listbox: TemplateChild::default(),
                qrcode_paintable: QRCodePaintable::new(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountDetailsPage {
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
        imp
            .qrcode_picture
            .set_paintable(Some(&imp.qrcode_paintable));
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
    }
}
