use crate::{
    models::{i18n, otp, Algorithm, OTPMethod, Provider, ProviderPatch, FAVICONS_PATH},
    widgets::{ErrorRevealer, ProviderImage},
};
use adw::prelude::*;
use gettextrs::gettext;
use glib::{clone, translate::IntoGlib};
use gtk::{gdk_pixbuf, gio, glib, subclass::prelude::*, CompositeTemplate};

mod imp {
    use super::*;
    use crate::models::OTPMethod;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_page.ui")]
    pub struct ProviderPage {
        pub actions: gio::SimpleActionGroup,
        pub methods_model: adw::EnumListModel,
        pub algorithms_model: adw::EnumListModel,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub error_revealer: TemplateChild<ErrorRevealer>,
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
        pub title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub delete_button: TemplateChild<gtk::Button>,
        pub selected_provider: RefCell<Option<Provider>>,
        // We need to hold a reference to the native file chooser
        pub file_chooser: RefCell<Option<gtk::FileChooserNative>>,
        pub selected_image: RefCell<Option<gio::File>>,
        #[template_child]
        pub back_btn: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProviderPage {
        const NAME: &'static str = "ProviderPage";
        type Type = super::ProviderPage;
        type ParentType = gtk::Box;

        fn new() -> Self {
            let methods_model = adw::EnumListModel::new(OTPMethod::static_type());
            let algorithms_model = adw::EnumListModel::new(Algorithm::static_type());

            Self {
                actions: gio::SimpleActionGroup::new(),
                image: TemplateChild::default(),
                revealer: TemplateChild::default(),
                error_revealer: TemplateChild::default(),
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
                delete_button: TemplateChild::default(),
                back_btn: TemplateChild::default(),
                methods_model,
                algorithms_model,
                selected_provider: RefCell::default(),
                file_chooser: RefCell::default(),
                selected_image: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            OTPMethod::static_type();
            Algorithm::static_type();
            klass.bind_template();

            klass.install_action("providers.save", None, move |page, _, _| {
                if let Err(err) = page.save() {
                    warn!("Failed to save provider {}", err);
                }
            });
            klass.install_action("providers.delete", None, move |page, _, _| {
                if let Err(err) = page.delete_provider() {
                    warn!("Failed to delete the provider {}", err);
                }
            });

            klass.install_action("providers.reset_image", None, move |page, _, _| {
                page.reset_image();
            });
            klass.install_action("providers.select_image", None, move |page, _, _| {
                page.open_select_image();
            });
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderPage {
        fn signals() -> &'static [Signal] {
            use once_cell::sync::Lazy;
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder(
                        "created",
                        &[Provider::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                    Signal::builder(
                        "updated",
                        &[Provider::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                    Signal::builder(
                        "deleted",
                        &[Provider::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            obj.action_set_enabled("providers.save", false);
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
    pub fn set_provider(&self, provider: Option<Provider>) {
        let imp = self.imp();
        if let Some(provider) = provider {
            imp.delete_button.show();
            imp.name_entry.set_text(&provider.name());
            imp.period_spinbutton.set_value(provider.period() as f64);

            if let Some(ref website) = provider.website() {
                imp.provider_website_entry.set_text(website);
            } else {
                imp.provider_website_entry.set_text("");
            }

            if let Some(ref website) = provider.help_url() {
                imp.provider_help_entry.set_text(website);
            } else {
                imp.provider_help_entry.set_text("");
            }

            imp.algorithm_comborow.set_selected(
                imp.algorithms_model
                    .find_position(provider.algorithm().into_glib()),
            );

            imp.default_counter_spinbutton
                .set_value(provider.default_counter() as f64);
            imp.digits_spinbutton.set_value(provider.digits() as f64);

            imp.method_comborow.set_selected(
                imp.methods_model
                    .find_position(provider.method().into_glib()),
            );
            imp.image.set_provider(Some(&provider));
            imp.title
                .set_title(&i18n::i18n_f("Editing Provider: {}", &[&provider.name()]));
            imp.selected_provider.replace(Some(provider));
        } else {
            imp.name_entry.set_text("");
            imp.delete_button.hide();
            imp.period_spinbutton
                .set_value(otp::TOTP_DEFAULT_PERIOD as f64);
            imp.provider_website_entry.set_text("");
            imp.provider_help_entry.set_text("");

            imp.algorithm_comborow.set_selected(
                imp.algorithms_model
                    .find_position(Algorithm::default().into_glib()),
            );

            imp.default_counter_spinbutton
                .set_value(otp::HOTP_DEFAULT_COUNTER as f64);
            imp.digits_spinbutton.set_value(otp::DEFAULT_DIGITS as f64);

            imp.method_comborow.set_selected(
                imp.methods_model
                    .find_position(OTPMethod::default().into_glib()),
            );
            imp.image.set_provider(None);
            imp.title.set_title(&gettext("New Provider"));
            imp.selected_provider.replace(None);
        }
    }

    // Validate the information typed by the user in order to enable/disable the save action
    // Note that we don't validate the urls other than: does `url` crate can parse it or not
    fn validate(&self) {
        let imp = self.imp();

        let provider_name = imp.name_entry.text();
        let provider_website = imp.provider_website_entry.text();
        let provider_help_url = imp.provider_help_entry.text();

        let is_valid = !provider_name.is_empty()
            && (provider_website.is_empty() || url::Url::parse(&provider_website).is_ok())
            && (provider_help_url.is_empty() || url::Url::parse(&provider_help_url).is_ok());

        self.action_set_enabled("providers.save", is_valid);
    }

    // Save the provider & emit a signal when one is created/updated
    fn save(&self) -> anyhow::Result<()> {
        let imp = self.imp();

        let name = imp.name_entry.text();
        let website = imp.provider_website_entry.text().to_string();
        let help_url = imp.provider_help_entry.text().to_string();
        let period = imp.period_spinbutton.value() as u32;
        let digits = imp.digits_spinbutton.value() as u32;
        let method = OTPMethod::from(imp.method_comborow.selected());
        let algorithm = Algorithm::from(imp.algorithm_comborow.selected());
        let default_counter = imp.default_counter_spinbutton.value() as u32;

        let image_uri = if let Some(file) = imp.selected_image.borrow().clone() {
            let basename = file.basename().unwrap();
            let icon_name = glib::base64_encode(basename.to_str().unwrap().as_bytes());
            let small_icon_name = format!("{icon_name}_32x32");
            let large_icon_name = format!("{icon_name}_96x96");

            // Create a 96x96 & 32x32 variants
            let stream = file.read(gio::Cancellable::NONE)?;
            let pixbuf = gdk_pixbuf::Pixbuf::from_stream(&stream, gio::Cancellable::NONE)?;
            log::debug!("Creating a 32x32 variant of the selected favicon");
            let small_pixbuf = pixbuf
                .scale_simple(32, 32, gdk_pixbuf::InterpType::Bilinear)
                .unwrap();
            small_pixbuf.savev(FAVICONS_PATH.join(small_icon_name), "png", &[])?;

            log::debug!("Creating a 96x96 variant of the selected favicon");
            let large_pixbuf = pixbuf
                .scale_simple(96, 96, gdk_pixbuf::InterpType::Bilinear)
                .unwrap();
            large_pixbuf.savev(FAVICONS_PATH.join(large_icon_name), "png", &[])?;

            Some(icon_name.to_string())
        } else {
            None
        };

        if let Some(provider) = imp.selected_provider.borrow().clone() {
            provider.update(&ProviderPatch {
                name: name.to_string(),
                website: Some(website),
                help_url: Some(help_url),
                image_uri,
                period: period as i32,
                digits: digits as i32,
                default_counter: default_counter as i32,
                algorithm: algorithm.to_string(),
                method: method.to_string(),
                is_backup_restore: false,
            })?;
            self.emit_by_name::<()>("updated", &[&provider]);
        } else {
            let provider = Provider::create(
                &name,
                period,
                algorithm,
                Some(website),
                method,
                digits,
                default_counter,
                Some(help_url),
                image_uri,
            )?;
            self.emit_by_name::<()>("created", &[&provider]);
        }
        Ok(())
    }

    fn open_select_image(&self) {
        let imp = self.imp();
        let parent = self.root().unwrap().downcast::<gtk::Window>().unwrap();

        let file_chooser = gtk::FileChooserNative::builder()
            .accept_label(&gettext("Select"))
            .cancel_label(&gettext("Cancel"))
            .modal(true)
            .action(gtk::FileChooserAction::Open)
            .transient_for(&parent)
            .build();

        let images_filter = gtk::FileFilter::new();
        images_filter.set_name(Some(&gettext("Image")));
        images_filter.add_pixbuf_formats();
        file_chooser.add_filter(&images_filter);

        file_chooser.connect_response(clone!(@weak self as page => move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let file = dialog.file().unwrap();
                page.set_image(file);
            }
            page.imp().file_chooser.replace(None);
            dialog.destroy();
        }));

        file_chooser.show();
        imp.file_chooser.replace(Some(file_chooser));
    }

    fn set_image(&self, file: gio::File) {
        let imp = self.imp();

        imp.image.set_from_file(&file);
        imp.selected_image.replace(Some(file));
    }

    fn reset_image(&self) {
        let imp = self.imp();
        imp.image.reset();
        imp.selected_image.replace(None);
    }

    fn delete_provider(&self) -> anyhow::Result<()> {
        let imp = self.imp();
        if let Some(provider) = imp.selected_provider.borrow().clone() {
            if provider.has_accounts() {
                imp.error_revealer.popup(&gettext(
                    "The provider has accounts assigned to it, please remove them first",
                ));
            } else if provider.delete().is_ok() {
                self.emit_by_name::<()>("deleted", &[&provider]);
            }
        } else {
            anyhow::bail!("Can't remove a provider as none are selected");
        }
        Ok(())
    }

    pub fn name_entry(&self) -> gtk::Entry {
        self.imp().name_entry.clone()
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        imp.algorithm_comborow
            .set_model(Some(&imp.algorithms_model));

        imp.method_comborow
            .connect_selected_item_notify(clone!(@weak self as page => move |_| {
                page.on_method_changed();
            }));

        let validate_cb = clone!(@weak self as page => move |_: &gtk::Entry| {
            page.validate();
        });

        imp.name_entry.connect_changed(validate_cb.clone());
        imp.provider_website_entry
            .connect_changed(validate_cb.clone());
        imp.provider_help_entry.connect_changed(validate_cb);

        imp.method_comborow.set_model(Some(&imp.methods_model));
    }

    fn on_method_changed(&self) {
        let imp = self.imp();

        let selected = OTPMethod::from(imp.method_comborow.selected());
        match selected {
            OTPMethod::TOTP => {
                imp.default_counter_row.hide();
                imp.period_row.show();
                imp.digits_spinbutton.set_value(otp::DEFAULT_DIGITS as f64);
                imp.period_spinbutton
                    .set_value(otp::TOTP_DEFAULT_PERIOD as f64);
            }
            OTPMethod::HOTP => {
                imp.default_counter_row.show();
                imp.period_row.hide();
                imp.default_counter_spinbutton
                    .set_value(otp::HOTP_DEFAULT_COUNTER as f64);
                imp.digits_spinbutton.set_value(otp::DEFAULT_DIGITS as f64);
            }
            OTPMethod::Steam => {
                imp.default_counter_row.hide();
                imp.period_row.show();
                imp.digits_spinbutton
                    .set_value(otp::STEAM_DEFAULT_DIGITS as f64);
                imp.period_spinbutton
                    .set_value(otp::STEAM_DEFAULT_PERIOD as f64);
                imp.algorithm_comborow
                    .set_selected(Algorithm::default().into_glib() as u32);
            }
        }

        imp.algorithm_comborow
            .set_sensitive(selected != OTPMethod::Steam);
        imp.period_row.set_sensitive(selected != OTPMethod::Steam);
        imp.digits_row.set_sensitive(selected != OTPMethod::Steam);
    }
}

impl Default for ProviderPage {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderPage")
    }
}
