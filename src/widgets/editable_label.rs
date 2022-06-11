use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use adw::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/editable_label.ui")]
    pub struct EditableLabel {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditableLabel {
        const NAME: &'static str = "EditableLabel";
        type Type = super::EditableLabel;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditableLabel {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.stack.set_visible_child_name("label");
        }
    }
    impl WidgetImpl for EditableLabel {
        fn grab_focus(&self, widget: &Self::Type) -> bool {
            self.parent_grab_focus(widget);
            if self.stack.visible_child_name().as_deref() == Some("entry") {
                self.entry.grab_focus();
                true
            } else {
                false
            }
        }
    }
    impl BinImpl for EditableLabel {}

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/editable_spin.ui")]
    pub struct EditableSpin {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditableSpin {
        const NAME: &'static str = "EditableSpin";
        type Type = super::EditableSpin;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditableSpin {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.stack.set_visible_child_name("label");
        }
    }
    impl WidgetImpl for EditableSpin {
        fn grab_focus(&self, widget: &Self::Type) -> bool {
            self.parent_grab_focus(widget);
            if self.stack.visible_child_name().as_deref() == Some("spin") {
                self.spin.grab_focus();
                true
            } else {
                false
            }
        }
    }
    impl BinImpl for EditableSpin {}
}

glib::wrapper! {
    pub struct EditableLabel(ObjectSubclass<imp::EditableLabel>)
        @extends gtk::Widget, adw::Bin;
}

impl EditableLabel {
    pub fn set_text(&self, text: &str) {
        let imp = self.imp();
        imp.label.set_text(text);
        imp.entry.set_text(text);
    }

    pub fn text(&self) -> glib::GString {
        self.imp().entry.text()
    }

    pub fn start_editing(&self) {
        self.imp().stack.set_visible_child_name("entry");
    }

    pub fn stop_editing(&self, commit: bool) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("label");
        if commit {
            imp.label.set_text(&imp.entry.text());
        }
    }
}

glib::wrapper! {
    pub struct EditableSpin(ObjectSubclass<imp::EditableSpin>)
        @extends gtk::Widget, adw::Bin;
}

impl EditableSpin {
    pub fn set_adjustment(&self, min: u32, max: u32) {
        self.imp().spin.set_adjustment(&gtk::Adjustment::new(
            0.0, min as f64, max as f64, 1.0, 1.0, 1.0,
        ));
    }

    pub fn set_text(&self, value: u32) {
        let imp = self.imp();
        imp.label.set_text(&value.to_string());
        imp.spin.set_value(value as f64);
    }

    pub fn start_editing(&self) {
        self.imp().stack.set_visible_child_name("spin");
    }

    pub fn stop_editing(&self, commit: bool) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("label");
        if commit {
            imp.label.set_text(&(imp.spin.value() as u32).to_string());
        }
    }

    pub fn value(&self) -> u32 {
        self.imp().spin.value() as u32
    }
}
