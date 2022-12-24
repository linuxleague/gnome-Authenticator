use adw::prelude::*;
use gtk::{
    glib::{self, clone},
    subclass::prelude::*,
};

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use glib::{ParamSpec, ParamSpecString, Value};
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default)]
    pub struct UrlRow {
        pub uri: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UrlRow {
        const NAME: &'static str = "UrlRow";
        type Type = super::UrlRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for UrlRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecString::builder("uri").build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    let uri = value.get().unwrap();
                    self.uri.replace(uri);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "uri" => self.uri.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let gesture = gtk::GestureClick::new();
            gesture.connect_pressed(clone!(@weak obj as row => move |_,_,_,_| {
                if let Some(ref uri) = *row.imp().uri.borrow() {
                    gtk::show_uri(gtk::Window::NONE, uri, 0);
                };
            }));

            obj.add_controller(&gesture);

            let image_suffix = gtk::Image::from_icon_name("link-symbolic");
            image_suffix.add_css_class("dim-label");
            obj.add_suffix(&image_suffix);
        }
    }
    impl WidgetImpl for UrlRow {}
    impl ListBoxRowImpl for UrlRow {}
    impl PreferencesRowImpl for UrlRow {}
    impl ActionRowImpl for UrlRow {}
}

glib::wrapper! {
    pub struct UrlRow(ObjectSubclass<imp::UrlRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl UrlRow {
    pub fn set_uri(&self, uri: &str) {
        self.set_subtitle(uri);
        self.imp().uri.borrow_mut().replace(uri.to_string());
    }
}
