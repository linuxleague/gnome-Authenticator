use adw::prelude::*;
use gtk::{
    gio,
    glib::{self, clone},
};

mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::UrlRow)]
    pub struct UrlRow {
        #[property(get, set = Self::set_uri)]
        pub uri: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UrlRow {
        const NAME: &'static str = "UrlRow";
        type Type = super::UrlRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for UrlRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let gesture = gtk::GestureClick::new();
            gesture.connect_pressed(clone!(@weak obj as row => move |_,_,_,_| {
                if let Some(uri) = row.imp().uri.borrow().clone() {
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(async move {
                        let file = gio::File::for_uri(&uri);
                        let launcher = gtk::FileLauncher::new(Some(&file));
                        if let Err(err) = launcher.launch_future(gtk::Window::NONE).await {
                            tracing::error!("Failed to open URI {err}");
                        }
                    });
                };
            }));

            obj.add_controller(gesture);

            let image_suffix = gtk::Image::from_icon_name("link-symbolic");
            image_suffix.add_css_class("dim-label");
            obj.add_suffix(&image_suffix);
        }
    }
    impl WidgetImpl for UrlRow {}
    impl ListBoxRowImpl for UrlRow {}
    impl PreferencesRowImpl for UrlRow {}
    impl ActionRowImpl for UrlRow {}

    impl UrlRow {
        pub fn set_uri(&self, uri: &str) {
            self.obj().set_subtitle(uri);
            self.uri.borrow_mut().replace(uri.to_owned());
        }
    }
}

glib::wrapper! {
    pub struct UrlRow(ObjectSubclass<imp::UrlRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}
