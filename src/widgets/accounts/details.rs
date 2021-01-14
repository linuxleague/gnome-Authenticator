use super::qrcode_paintable::QRCodePaintable;
use crate::{models::Account, widgets::UrlRow};
use gtk::subclass::prelude::*;
use gtk::{glib, prelude::*, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass;

    #[derive(Debug, CompositeTemplate)]
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

    impl ObjectSubclass for AccountDetailsPage {
        const NAME: &'static str = "AccountDetailsPage";
        type Type = super::AccountDetailsPage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

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
            UrlRow::static_type();
            klass.set_template_from_resource(
                "/com/belmoussaoui/Authenticator/account_details_page.ui",
            );
            Self::bind_template_children(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
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
        let self_ = imp::AccountDetailsPage::from_instance(self);
        self_
            .qrcode_picture
            .set_paintable(Some(&self_.qrcode_paintable));
    }

    pub fn set_account(&self, account: &Account) {
        let self_ = imp::AccountDetailsPage::from_instance(self);
        let qr_code = account.qr_code();
        self_.qrcode_paintable.set_qrcode(qr_code);

        self_.account_label.set_text(&account.name());
        self_.provider_label.set_text(&account.provider().name());

        if let Some(ref website) = account.provider().website() {
            self_.website_row.set_uri(website);
            self_.website_row.show();
        } else {
            self_.website_row.hide();
        }
    }
}
