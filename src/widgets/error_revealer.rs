use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/error_revealer.ui")]
    pub struct ErrorRevealer {
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorRevealer {
        const NAME: &'static str = "ErrorRevealer";
        type Type = super::ErrorRevealer;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ErrorRevealer {
        fn dispose(&self, _obj: &Self::Type) {
            self.revealer.unparent();
            self.label.unparent();
        }
    }

    impl WidgetImpl for ErrorRevealer {}
}

glib::wrapper! {
    pub struct ErrorRevealer(ObjectSubclass<imp::ErrorRevealer>) @extends gtk::Widget;
}

impl ErrorRevealer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    pub fn popup(&self, text: &str) {
        let self_ = imp::ErrorRevealer::from_instance(self);
        self_.label.set_text(text);

        self_.revealer.set_reveal_child(true);
        glib::timeout_add_seconds_local(
            2,
            glib::clone!(@weak self as error_revealer => move || {
                let error_revealer_ = imp::ErrorRevealer::from_instance(&error_revealer);
                error_revealer_.revealer.set_reveal_child(false);
                glib::Continue(false)
            }),
        );
    }
}
