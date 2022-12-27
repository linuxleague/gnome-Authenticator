use gtk::{glib, prelude::*, subclass::prelude::*};

#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "CameraItem")]
pub struct CameraItem {
    pub nick: String,
    pub node_id: u32,
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct CameraRow {
        pub label: gtk::Label,
        pub checkmark: gtk::Image,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraRow {
        const NAME: &'static str = "CameraRow";
        type Type = super::CameraRow;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for CameraRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_spacing(6);
            self.checkmark.set_icon_name(Some("object-select-symbolic"));
            self.checkmark.hide();

            obj.append(&self.label);
            obj.append(&self.checkmark);
        }
    }
    impl WidgetImpl for CameraRow {}
    impl BoxImpl for CameraRow {}
}

glib::wrapper! {
    pub struct CameraRow(ObjectSubclass<imp::CameraRow>)
        @extends gtk::Widget, gtk::Box;
}

impl Default for CameraRow {
    fn default() -> Self {
        glib::Object::new(&[])
    }
}

impl CameraRow {
    pub fn set_label(&self, label: &str) {
        self.imp().label.set_label(label);
    }

    pub fn set_selected(&self, selected: bool) {
        self.imp().checkmark.set_visible(selected);
    }

    pub fn set_item(&self, item: &CameraItem) {
        self.imp().label.set_label(&item.nick);
    }
}
