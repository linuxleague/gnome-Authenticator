use crate::models::{Algorithm, Provider};
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::translate::ToGlib;
use glib::{glib_object_subclass, glib_wrapper};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::{ComboRowExt, EnumListModelExt};

pub enum ProviderPageMode {
    Create,
    Edit,
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    #[derive(Debug, CompositeTemplate)]
    pub struct ProviderPage {
        #[template_child(id = "name_entry")]
        pub name_entry: TemplateChild<gtk::Entry>,
        #[template_child(id = "period_spinbutton")]
        pub period_spinbutton: TemplateChild<gtk::SpinButton>,
        #[template_child(id = "provider_website_entry")]
        pub provider_website_entry: TemplateChild<gtk::Entry>,
        #[template_child(id = "provider_help_entry")]
        pub provider_help_entry: TemplateChild<gtk::Entry>,
        #[template_child(id = "image_stack")]
        pub image_stack: TemplateChild<gtk::Stack>,
        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child(id = "algorithm_comborow")]
        pub algorithm_comborow: TemplateChild<libhandy::ComboRow>,
        #[template_child(id = "title")]
        pub title: TemplateChild<gtk::Label>,
        pub algorithms_model: libhandy::EnumListModel,
    }

    impl ObjectSubclass for ProviderPage {
        const NAME: &'static str = "ProviderPage";
        type Type = super::ProviderPage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let algorithms_model = libhandy::EnumListModel::new(Algorithm::static_type());

            Self {
                name_entry: TemplateChild::default(),
                period_spinbutton: TemplateChild::default(),
                provider_website_entry: TemplateChild::default(),
                provider_help_entry: TemplateChild::default(),
                image_stack: TemplateChild::default(),
                spinner: TemplateChild::default(),
                algorithm_comborow: TemplateChild::default(),
                title: TemplateChild::default(),
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

        self_.image_stack.get().set_visible_child_name("loading");
        self_.spinner.get().start();

        self_.algorithm_comborow.get().set_selected(
            self_
                .algorithms_model
                .find_position(provider.algorithm().to_glib()),
        );

        /*let sender = self.sender.clone();
        spawn!(async move {
            if let Ok(file) = p.favicon().await {
                send!(sender, AddAccountAction::SetIcon(file));
            }
        });*/

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

                self_.image_stack.get().set_visible_child_name("image");
                self_.spinner.get().stop();
                self_.algorithm_comborow.get().set_selected(0);
            }
            ProviderPageMode::Edit => {}
        }
    }
}
