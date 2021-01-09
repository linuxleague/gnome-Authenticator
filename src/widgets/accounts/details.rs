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
        pub image: TemplateChild<gtk::Image>,
        #[template_child]
        pub provider_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub account_label: TemplateChild<gtk::Label>,
        #[template_child(id = "list")]
        pub listbox: TemplateChild<gtk::ListBox>,
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
                image: TemplateChild::default(),
                account_label: TemplateChild::default(),
                provider_label: TemplateChild::default(),
                website_row: TemplateChild::default(),
                listbox: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            UrlRow::static_type();
            klass.set_template_from_resource(
                "/com/belmoussaoui/Authenticator/account_details_page.ui",
            );
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for AccountDetailsPage {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
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

    pub fn set_account(&self, account: &Account) {
        let self_ = imp::AccountDetailsPage::from_instance(self);
        let is_dark = gtk::Settings::get_default()
            .unwrap()
            .get_property_gtk_application_prefer_dark_theme();
        let qr_code = account.qr_code(is_dark).unwrap();

        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file(qr_code.get_path().unwrap()).unwrap();
        self_.image.get().set_from_pixbuf(Some(&pixbuf));

        self_.account_label.get().set_text(&account.name());
        self_
            .provider_label
            .get()
            .set_text(&account.provider().name());

        if let Some(ref website) = account.provider().website() {
            self_.website_row.get().set_uri(website);
            self_.website_row.get().show();
        } else {
            self_.website_row.get().hide();
        }
    }
}
