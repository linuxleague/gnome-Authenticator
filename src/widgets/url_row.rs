use gio::subclass::ObjectSubclass;
use glib::{clone, glib_wrapper, Cast, ObjectExt, StaticType, ToValue};
use gtk::WidgetExt;
use libhandy::ActionRowExt;

mod imp {
    use super::*;
    use glib::{glib_object_subclass, subclass};
    use gtk::subclass::prelude::*;
    use libhandy::subclass::action_row::ActionRowImpl;
    use std::cell::RefCell;

    static PROPERTIES: [subclass::Property; 1] = [subclass::Property("uri", |name| {
        glib::ParamSpec::string(
            name,
            "uri",
            "The Row URI",
            None,
            glib::ParamFlags::READWRITE,
        )
    })];

    pub struct UrlRow {
        pub uri: RefCell<Option<String>>,
    }

    impl ObjectSubclass for UrlRow {
        const NAME: &'static str = "UrlRow";
        type Type = super::UrlRow;
        type ParentType = libhandy::ActionRow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            Self {
                uri: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.install_properties(&PROPERTIES);
        }
    }

    impl ObjectImpl for UrlRow {
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("uri", ..) => {
                    let uri = value.get().unwrap();
                    self.uri.replace(uri);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];
            match *prop {
                subclass::Property("uri", ..) => self.uri.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for UrlRow {}
    impl ListBoxRowImpl for UrlRow {}
    impl ActionRowImpl for UrlRow {}
}

glib_wrapper! {
    pub struct UrlRow(ObjectSubclass<imp::UrlRow>) @extends gtk::Widget, gtk::ListBoxRow, libhandy::ActionRow;
}

impl UrlRow {
    #[allow(clippy::new_without_default)]
    pub fn new(title: &str, icon_name: &str) -> Self {
        let row = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create UrlRow")
            .downcast::<UrlRow>()
            .expect("Created object is of wrong type");
        row.setup_widgets(title, icon_name);
        row
    }

    fn setup_widgets(&self, title: &str, icon_name: &str) {
        self.set_property("title", &title).unwrap();

        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(clone!(@weak self as row => move |_,_,_,_| {
            row.open_uri();
        }));

        self.add_controller(&gesture);

        let image = gtk::Image::from_icon_name(Some(icon_name));
        self.add_suffix(&image);
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
