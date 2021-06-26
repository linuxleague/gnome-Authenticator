use adw::prelude::*;
use glib::{clone, ToValue};
use gtk::prelude::*;
use gtk::{glib, subclass::prelude::*};

mod imp {
    use super::*;
    use adw::subclass::action_row::ActionRowImpl;
    use glib::ParamSpec;
    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct UrlRow {
        pub uri: RefCell<Option<String>>,
        pub icon_name: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UrlRow {
        const NAME: &'static str = "UrlRow";
        type Type = super::UrlRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for UrlRow {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_string(
                        "uri",
                        "uri",
                        "The Row URI",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "icon-name",
                        "icon name",
                        "The Icon Name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "uri" => {
                    let uri = value.get().unwrap();
                    self.uri.replace(uri);
                }
                "icon-name" => {
                    let icon_name = value.get().unwrap();
                    self.icon_name.replace(icon_name);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "uri" => self.uri.borrow().to_value(),
                "icon-name" => self.icon_name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for UrlRow {}
    impl ListBoxRowImpl for UrlRow {}
    impl ActionRowImpl for UrlRow {}
}

glib::wrapper! {
    pub struct UrlRow(ObjectSubclass<imp::UrlRow>) @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow;
}

impl UrlRow {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create UrlRow")
    }

    fn setup_widgets(&self) {
        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(clone!(@weak self as row => move |_,_,_,_| {
            row.open_uri();
        }));

        self.add_controller(&gesture);

        let image_prefix = gtk::Image::from_icon_name(Some("image-missing-symbolic"));
        self.bind_property("icon-name", &image_prefix, "icon-name")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        self.add_prefix(&image_prefix);

        let image_suffix = gtk::Image::from_icon_name(Some("link-symbolic"));
        image_suffix.add_css_class("dim-label");
        self.add_suffix(&image_suffix);
    }

    fn open_uri(&self) {
        let self_ = imp::UrlRow::from_instance(self);
        if let Some(ref uri) = *self_.uri.borrow() {
            gtk::show_uri(gtk::NONE_WINDOW, uri, 0);
        }
    }

    pub fn set_uri(&self, uri: &str) {
        self.set_subtitle(Some(uri));
        let self_ = imp::UrlRow::from_instance(self);
        self_.uri.borrow_mut().replace(uri.to_string());
    }
}
