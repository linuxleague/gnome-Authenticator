use crate::{
    models::{Algorithm, OTPMethod, Provider},
    widgets::ProviderImage,
};
use adw::ComboRowExt;
use gettextrs::gettext;
use glib::{clone, translate::ToGlib};
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate};

pub enum ProviderPageMode {
    Create,
    Edit,
}

mod imp {
    use crate::models::OTPMethod;

    use super::*;
    use glib::subclass;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_page.ui")]
    pub struct ProviderPage {
        pub methods_model: adw::EnumListModel,
        pub algorithms_model: adw::EnumListModel,
        #[template_child]
        pub image: TemplateChild<ProviderImage>,
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
        pub method_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub algorithm_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub period_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub digits_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub default_counter_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
    }

    impl ObjectSubclass for ProviderPage {
        const NAME: &'static str = "ProviderPage";
        type Type = super::ProviderPage;
        type ParentType = gtk::Box;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let methods_model = adw::EnumListModel::new(OTPMethod::static_type());
            let algorithms_model = adw::EnumListModel::new(Algorithm::static_type());

            Self {
                image: TemplateChild::default(),
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
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderPage {
        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for ProviderPage {}
    impl BoxImpl for ProviderPage {}
}

glib::wrapper! {
    pub struct ProviderPage(ObjectSubclass<imp::ProviderPage>) @extends gtk::Widget, gtk::Box;
}
impl ProviderPage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderPage")
    }

    pub fn set_provider(&self, provider: Provider) {
        let self_ = imp::ProviderPage::from_instance(self);
        self_.name_entry.set_text(&provider.name());
        self_.period_spinbutton.set_value(provider.period() as f64);

        if let Some(ref website) = provider.website() {
            self_.provider_website_entry.set_text(website);
        }

        if let Some(ref website) = provider.help_url() {
            self_.provider_help_entry.set_text(website);
        }

        self_.algorithm_comborow.set_selected(
            self_
                .algorithms_model
                .find_position(provider.algorithm().to_glib()),
        );
        self.on_method_changed();

        self_
            .default_counter_spinbutton
            .set_value(provider.default_counter() as f64);
        self_.digits_spinbutton.set_value(provider.digits() as f64);

        self_.method_comborow.set_selected(
            self_
                .methods_model
                .find_position(provider.method().to_glib()),
        );
        self_.image.set_provider(&provider);
        self_
            .title
            .set_text(&format!("Editing provider: {}", provider.name()));
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderPage::from_instance(self);
        self_
            .algorithm_comborow
            .set_model(Some(&self_.algorithms_model));

        self_.method_comborow.connect_property_selected_item_notify(
            clone!(@weak self as page => move |_| {
                page.on_method_changed();
            }),
        );
        self_.method_comborow.set_model(Some(&self_.methods_model));
    }

    fn on_method_changed(&self) {
        let self_ = imp::ProviderPage::from_instance(self);

        let selected = OTPMethod::from(self_.method_comborow.get_selected());
        match selected {
            OTPMethod::TOTP => {
                self_.default_counter_row.hide();
                self_.period_row.show();
            }
            OTPMethod::HOTP => {
                self_.default_counter_row.show();
                self_.period_row.hide();
            }
            OTPMethod::Steam => {}
        }
    }

    pub fn set_mode(&self, mode: ProviderPageMode) {
        let self_ = imp::ProviderPage::from_instance(self);
        match mode {
            ProviderPageMode::Create => {
                self_.title.set_label(&gettext("New Provider"));
                self_.name_entry.set_text("");
                self_.period_spinbutton.set_value(30_f64);
                self_.provider_website_entry.set_text("");
                self_.provider_help_entry.set_text("");

                self_.method_comborow.set_selected(0);
            }
            ProviderPageMode::Edit => {}
        }
    }
}
