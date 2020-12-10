use crate::models::{Algorithm, OTPMethod, Provider};
use crate::widgets::{ProviderImage, ProviderImageSize};
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::translate::ToGlib;
use glib::{clone, glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::ComboRowExt;

pub enum ProviderPageMode {
    Create,
    Edit,
}

mod imp {
    use crate::models::OTPMethod;

    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    pub struct ProviderPage {
        pub image: ProviderImage,
        pub methods_model: libhandy::EnumListModel,
        pub algorithms_model: libhandy::EnumListModel,
        #[template_child]
        pub main_container: TemplateChild<gtk::Box>,
        #[template_child]
        pub name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub period_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub digits_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub default_counter_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub provider_website_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub provider_help_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub method_comborow: TemplateChild<libhandy::ComboRow>,
        #[template_child]
        pub algorithm_comborow: TemplateChild<libhandy::ComboRow>,
        #[template_child]
        pub period_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub digits_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub default_counter_row: TemplateChild<libhandy::ActionRow>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
    }

    impl ObjectSubclass for ProviderPage {
        const NAME: &'static str = "ProviderPage";
        type Type = super::ProviderPage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let methods_model = libhandy::EnumListModel::new(OTPMethod::static_type());
            let algorithms_model = libhandy::EnumListModel::new(Algorithm::static_type());

            Self {
                image: ProviderImage::new(ProviderImageSize::Large),
                main_container: TemplateChild::default(),
                name_entry: TemplateChild::default(),
                period_spinbutton: TemplateChild::default(),
                digits_spinbutton: TemplateChild::default(),
                default_counter_spinbutton: TemplateChild::default(),
                provider_website_entry: TemplateChild::default(),
                provider_help_entry: TemplateChild::default(),
                method_comborow: TemplateChild::default(),
                algorithm_comborow: TemplateChild::default(),
                period_row: TemplateChild::default(),
                digits_row: TemplateChild::default(),
                default_counter_row: TemplateChild::default(),
                title: TemplateChild::default(),
                methods_model,
                algorithms_model,
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/provider_page.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for ProviderPage {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProviderPage {}
    impl BoxImpl for ProviderPage {}
}

glib_wrapper! {
    pub struct ProviderPage(ObjectSubclass<imp::ProviderPage>) @extends gtk::Widget, gtk::Box;
}
impl ProviderPage {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProviderPage")
            .downcast::<ProviderPage>()
            .expect("Created object is of wrong type")
    }

    pub fn set_provider(&self, provider: Provider) {
        let self_ = imp::ProviderPage::from_instance(self);
        self_.name_entry.get().set_text(&provider.name());
        self_
            .period_spinbutton
            .get()
            .set_value(provider.period() as f64);

        if let Some(ref website) = provider.website() {
            self_.provider_website_entry.get().set_text(website);
        }

        if let Some(ref website) = provider.help_url() {
            self_.provider_help_entry.get().set_text(website);
        }

        self_.algorithm_comborow.get().set_selected(
            self_
                .algorithms_model
                .find_position(provider.algorithm().to_glib()),
        );
        self.on_algorithm_changed();

        self_
            .default_counter_spinbutton
            .get()
            .set_value(provider.default_counter() as f64);
        self_
            .digits_spinbutton
            .get()
            .set_value(provider.digits() as f64);

        self_.method_comborow.get().set_selected(
            self_
                .methods_model
                .find_position(provider.method().to_glib()),
        );
        self_.image.set_provider(&provider);
        self_
            .title
            .get()
            .set_text(&format!("Editing provider: {}", provider.name()));
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderPage::from_instance(self);
        self_
            .algorithm_comborow
            .get()
            .set_model(Some(&self_.algorithms_model));

        self_.main_container.get().prepend(&self_.image);

        self_
            .algorithm_comborow
            .get()
            .connect_property_selected_item_notify(clone!(@weak self as page => move |_| {
                page.on_algorithm_changed();
            }));
        self_
            .method_comborow
            .get()
            .set_model(Some(&self_.methods_model));
    }

    fn on_algorithm_changed(&self) {
        let self_ = imp::ProviderPage::from_instance(self);

        let selected = OTPMethod::from(self_.method_comborow.get().get_selected());
        match selected {
            OTPMethod::TOTP => {
                self_.default_counter_row.get().hide();
                self_.period_row.get().show();
            }
            OTPMethod::HOTP => {
                self_.default_counter_row.get().show();
                self_.period_row.get().hide();
            }
            OTPMethod::Steam => {}
        }
    }

    pub fn set_mode(&self, mode: ProviderPageMode) {
        let self_ = imp::ProviderPage::from_instance(self);
        match mode {
            ProviderPageMode::Create => {
                self_.title.get().set_label("New Provider");
                self_.name_entry.get().set_text("");
                self_.period_spinbutton.get().set_value(30_f64);
                self_.provider_website_entry.get().set_text("");
                self_.provider_help_entry.get().set_text("");

                self_.method_comborow.get().set_selected(0);
            }
            ProviderPageMode::Edit => {}
        }
    }
}
