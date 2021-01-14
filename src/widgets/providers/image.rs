use crate::models::Provider;
use glib::{clone, Receiver, Sender};
use gtk::subclass::prelude::*;
use gtk::{gio, glib, prelude::*, CompositeTemplate};

pub enum ImageAction {
    Ready(gio::File),
    Failed,
}

mod imp {
    use super::*;
    use glib::subclass;
    use std::cell::{Cell, RefCell};

    static PROPERTIES: [subclass::Property; 2] = [
        subclass::Property("provider", |name| {
            glib::ParamSpec::object(
                name,
                "provider",
                "Provider",
                Provider::static_type(),
                glib::ParamFlags::READWRITE,
            )
        }),
        subclass::Property("size", |name| {
            glib::ParamSpec::uint(
                name,
                "size",
                "Image size",
                24,
                96,
                48,
                glib::ParamFlags::READWRITE,
            )
        }),
    ];

    #[derive(Debug, CompositeTemplate)]
    pub struct ProviderImage {
        pub size: Cell<u32>,
        pub sender: Sender<ImageAction>,
        pub receiver: RefCell<Option<Receiver<ImageAction>>>,
        pub provider: RefCell<Option<Provider>>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub image: TemplateChild<gtk::Image>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
    }

    impl ObjectSubclass for ProviderImage {
        const NAME: &'static str = "ProviderImage";
        type Type = super::ProviderImage;
        type ParentType = gtk::Box;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));
            Self {
                sender,
                receiver,
                size: Cell::new(96),
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
                subclass::Property("size", ..) => {
                    let size = value.get().unwrap().unwrap();
                    self.size.set(size);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                subclass::Property("size", ..) => self.size.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for ProviderImage {}
    impl BoxImpl for ProviderImage {}
}

glib::wrapper! {
    pub struct ProviderImage(ObjectSubclass<imp::ProviderImage>) @extends gtk::Widget, gtk::Box;
}
impl ProviderImage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ProviderImage")
    }

    pub fn set_provider(&self, provider: &Provider) {
        let self_ = imp::ProviderImage::from_instance(self);
        self_.stack.set_visible_child_name("loading");
        self_.spinner.start();

        self.set_property("provider", &provider.clone()).unwrap();

        match provider.image_uri() {
            Some(uri) => {
                // Very dirty hack to store that we couldn't find an icon
                // to avoid re-hitting the website every time we have to display it
                if uri == "invalid" {
                    self_
                        .image
                        .set_from_icon_name(Some("image-missing-symbolic"));
                    self_.stack.set_visible_child_name("image");
                    return;
                }

                let file = gio::File::new_for_uri(&uri);
                if !file.query_exists(gio::NONE_CANCELLABLE) {
                    self.fetch();
                    return;
                }

                self_.image.set_from_file(file.get_path().unwrap());
                self_.stack.set_visible_child_name("image");
            }
            _ => {
                self.fetch();
            }
        }
    }

    fn fetch(&self) {
        let self_ = imp::ProviderImage::from_instance(self);
        let sender = self_.sender.clone();
        self_.stack.set_visible_child_name("loading");
        self_.spinner.start();
        let p = self.provider();
        gtk_macros::spawn!(async move {
            match p.favicon().await {
                Ok(file) => gtk_macros::send!(sender, ImageAction::Ready(file)),
                Err(_) => gtk_macros::send!(sender, ImageAction::Failed),
            }
        });
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
        self.bind_property("size", &*self_.image, "pixel-size")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    fn do_action(&self, action: ImageAction) -> glib::Continue {
        let self_ = imp::ProviderImage::from_instance(self);
        match action {
            ImageAction::Failed => {
                self_
                    .image
                    .set_from_icon_name(Some("image-missing-symbolic"));
                self.provider().set_image_uri("invalid");
            }
            ImageAction::Ready(image) => {
                self_.image.set_from_file(image.get_path().unwrap());
                self.provider().set_image_uri(&image.get_uri());
            }
        }
        self_.stack.set_visible_child_name("image");
        self_.spinner.stop();

        glib::Continue(true)
    }
}
