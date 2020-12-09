use crate::models::Provider;
use gio::{subclass::ObjectSubclass, FileExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use glib::{Receiver, Sender};
use gtk::{prelude::*, CompositeTemplate};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProviderImageSize {
    Small,
    Large,
}

pub enum ImageAction {
    Ready(gio::File),
    Failed,
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    static PROPERTIES: [subclass::Property; 1] = [subclass::Property("provider", |name| {
        glib::ParamSpec::object(
            name,
            "provider",
            "Provider",
            Provider::static_type(),
            glib::ParamFlags::READWRITE,
        )
    })];

    #[derive(Debug, CompositeTemplate)]
    pub struct ProviderImage {
        pub sender: Sender<ImageAction>,
        pub receiver: RefCell<Option<Receiver<ImageAction>>>,
        #[template_child(id = "stack")]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child(id = "image")]
        pub image: TemplateChild<gtk::Image>,
        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<gtk::Spinner>,
        pub provider: RefCell<Option<Provider>>,
    }

    impl ObjectSubclass for ProviderImage {
        const NAME: &'static str = "ProviderImage";
        type Type = super::ProviderImage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));
            Self {
                sender,
                receiver,
                stack: TemplateChild::default(),
                image: TemplateChild::default(),
                spinner: TemplateChild::default(),
                provider: RefCell::new(None),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/provider_image.ui");
            Self::bind_template_children(klass);
            klass.install_properties(&PROPERTIES);
        }
    }

    impl ObjectImpl for ProviderImage {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("provider", ..) => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for ProviderImage {}
    impl BoxImpl for ProviderImage {}
}

glib_wrapper! {
    pub struct ProviderImage(ObjectSubclass<imp::ProviderImage>) @extends gtk::Widget, gtk::Box;
}
impl ProviderImage {
    pub fn new(image_size: ProviderImageSize) -> Self {
        let image = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create ProviderImage")
            .downcast::<ProviderImage>()
            .expect("Created ProviderImage is of wrong type");
        image.set_size(image_size);
        image
    }

    pub fn set_provider(&self, provider: &Provider) {
        let self_ = imp::ProviderImage::from_instance(self);
        self_.stack.get().set_visible_child_name("loading");
        self_.spinner.get().start();

        self.set_property("provider", &provider.clone()).unwrap();

        match provider.image_uri() {
            Some(uri) => {
                // Very dirty hack to store that we couldn't find an icon
                // to avoid re-hitting the website every time we have to display it
                if uri == "invalid" {
                    self_
                        .image
                        .get()
                        .set_from_icon_name(Some("image-missing-symbolic"));
                    self_.stack.get().set_visible_child_name("image");
                    return;
                }

                let file = gio::File::new_for_uri(&uri);
                if !file.query_exists(gio::NONE_CANCELLABLE) {
                    self.fetch();
                    return;
                }

                self_.image.get().set_from_file(file.get_path().unwrap());
                self_.stack.get().set_visible_child_name("image");
            }
            _ => {
                self.fetch();
            }
        }
    }

    fn fetch(&self) {
        let self_ = imp::ProviderImage::from_instance(self);
        let sender = self_.sender.clone();
        self_.stack.get().set_visible_child_name("loading");
        self_.spinner.get().start();
        let p = self.provider();
        spawn!(async move {
            match p.favicon().await {
                Ok(file) => send!(sender, ImageAction::Ready(file)),
                Err(_) => send!(sender, ImageAction::Failed),
            }
        });
    }

    pub fn set_size(&self, image_size: ProviderImageSize) {
        let self_ = imp::ProviderImage::from_instance(self);

        match image_size {
            ProviderImageSize::Small => {
                self_.image.get().set_pixel_size(48);
                self.set_halign(gtk::Align::Start);
            }
            ProviderImageSize::Large => {
                self_.image.get().set_pixel_size(96);
                self.set_halign(gtk::Align::Center);
            }
        }
    }

    fn provider(&self) -> Provider {
        let provider = self.get_property("provider").unwrap();
        provider.get::<Provider>().unwrap().unwrap()
    }

    fn setup_widgets(&self) {
        let self_ = imp::ProviderImage::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as image => move |action| image.do_action(action)),
        );
    }

    fn do_action(&self, action: ImageAction) -> glib::Continue {
        let self_ = imp::ProviderImage::from_instance(self);
        match action {
            ImageAction::Failed => {
                self_
                    .image
                    .get()
                    .set_from_icon_name(Some("image-missing-symbolic"));
                self.provider().set_image_uri("invalid");
            }
            ImageAction::Ready(image) => {
                self_.image.get().set_from_file(image.get_path().unwrap());
                self.provider().set_image_uri(&image.get_uri());
            }
        }
        self_.stack.get().set_visible_child_name("image");
        self_.spinner.get().stop();

        glib::Continue(true)
    }
}
