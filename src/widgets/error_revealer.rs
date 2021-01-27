const REVEAL_TIME_SECS: u32 = 2;

use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use glib::SourceId;
    use gtk::CompositeTemplate;

    use std::cell::RefCell;

    use super::*;
    use glib::subclass;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/error_revealer.ui")]
    pub struct ErrorRevealer {
        pub source_id: RefCell<Option<SourceId>>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
    }

    impl ObjectSubclass for ErrorRevealer {
        const NAME: &'static str = "ErrorRevealer";
        type ParentType = gtk::Widget;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;
        type Type = super::ErrorRevealer;

        glib::object_subclass!();

        fn new() -> Self {
            Self {
                source_id: RefCell::default(),
                label: TemplateChild::default(),
                revealer: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ErrorRevealer {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }

        fn dispose(&self, _obj: &Self::Type) {
            if let Some(id) = self.source_id.borrow_mut().take() {
                glib::source_remove(id);
            }
            self.revealer.unparent();
            self.label.unparent();
        }
    }

    impl WidgetImpl for ErrorRevealer {}
}

glib::wrapper! {
    pub struct ErrorRevealer(
        ObjectSubclass<imp::ErrorRevealer>)
        @extends gtk::Widget, gtk::ListBoxRow;
}

impl ErrorRevealer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    pub fn popup(&self, text: &str) {
        let self_ = imp::ErrorRevealer::from_instance(self);
        self_.label.set_text(text);

        if let Some(id) = self_.source_id.borrow_mut().take() {
            glib::source_remove(id);
        }

        self_.revealer.set_reveal_child(true);
        let id = glib::timeout_add_seconds_local(
            REVEAL_TIME_SECS,
            glib::clone!(@weak self as error_revealer => move || {
                let error_revealer_ = imp::ErrorRevealer::from_instance(&error_revealer);
                error_revealer_.revealer.set_reveal_child(false);
                glib::Continue(false)
            }),
        );

        self_.source_id.replace(Some(id));
    }
}
