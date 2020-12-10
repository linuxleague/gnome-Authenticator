use crate::models::Account;
use gio::{subclass::ObjectSubclass, FileExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::prelude::*;

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    pub struct QRCodePage {
        #[template_child]
        pub image: TemplateChild<gtk::Image>,
        #[template_child]
        pub provider_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub account_label: TemplateChild<gtk::Label>,
        #[template_child(id = "provider_website_row")]
        pub website_row: TemplateChild<libhandy::ActionRow>,
    }

    impl ObjectSubclass for QRCodePage {
        const NAME: &'static str = "QRCodePage";
        type Type = super::QRCodePage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            Self {
                image: TemplateChild::default(),
                account_label: TemplateChild::default(),
                provider_label: TemplateChild::default(),
                website_row: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource(
                "/com/belmoussaoui/Authenticator/account_qrcode_page.ui",
            );
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for QRCodePage {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for QRCodePage {}
    impl BoxImpl for QRCodePage {}
}

glib_wrapper! {
    pub struct QRCodePage(ObjectSubclass<imp::QRCodePage>) @extends gtk::Widget, gtk::Box;
}
impl QRCodePage {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create QRCodePage")
            .downcast::<QRCodePage>()
            .expect("Created object is of wrong type")
    }

    pub fn set_account(&self, account: &Account) {
        let self_ = imp::QRCodePage::from_instance(self);
        let is_dark = gtk::Settings::get_default()
            .unwrap()
            .get_property_gtk_application_prefer_dark_theme();
        let qr_code = account.qr_code(is_dark).unwrap();

        let pixbuf = gdk_pixbuf::Pixbuf::from_file(qr_code.get_path().unwrap()).unwrap();
        self_.image.get().set_from_pixbuf(Some(&pixbuf));

        self_.account_label.get().set_text(&account.name());
        self_
            .provider_label
            .get()
            .set_text(&account.provider().name());

        if let Some(ref website) = account.provider().website() {
            self_.website_row.get().set_subtitle(Some(website));
            self_.website_row.get().show();
        } else {
            self_.website_row.get().hide();
        }
    }
}
