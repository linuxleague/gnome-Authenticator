use crate::models::Provider;
use glib::{clone, Receiver, Sender};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::{send, spawn};

pub enum ImageAction {
    Ready(gio::File),
    Failed,
}

mod imp {
    use super::*;
    use glib::{subclass, ParamSpec};
    use std::cell::{Cell, RefCell};

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_image.ui")]
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
        type Interfaces = ();
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
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderImage {
        fn constructed(&self, obj: &Self::Type) {
            obj.setup_widgets();
            self.parent_constructed(obj);
        }
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;

            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::object(
                        "provider",
                        "provider",
                        "Provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::uint(
                        "size",
                        "size",
                        "Image size",
                        24,
                        96,
                        48,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
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
            match pspec.get_name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                "size" => {
                    let size = value.get().unwrap().unwrap();
                    self.size.set(size);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "provider" => self.provider.borrow().to_value(),
                "size" => self.size.get().to_value(),
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

    pub fn set_provider(&self, provider: Option<&Provider>) {
        let self_ = imp::ProviderImage::from_instance(self);
        if let Some(provider) = provider {
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
        } else {
            self_
                .image
                .set_from_icon_name(Some("image-missing-symbolic"));
        }
    }

    fn fetch(&self) {
        let self_ = imp::ProviderImage::from_instance(self);
        let sender = self_.sender.clone();
        self_.stack.set_visible_child_name("loading");
        self_.spinner.start();
        let p = self.provider();
        spawn!(async move {
            match p.favicon().await {
                Ok(file) => send!(sender, ImageAction::Ready(file)),
                Err(_) => send!(sender, ImageAction::Failed),
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
        let result = match action {
            //TODO: handle network failure and other errors differently
            ImageAction::Failed => {
                self_
                    .image
                    .set_from_icon_name(Some("image-missing-symbolic"));
                self.provider().set_image_uri("invalid")
            }
            ImageAction::Ready(image) => {
                self_.image.set_from_file(image.get_path().unwrap());
                self.provider().set_image_uri(&image.get_uri())
            }
        };
        if let Err(err) = result {
            warn!("Failed to update the provider image {}", err);
        }
        self_.stack.set_visible_child_name("image");
        self_.spinner.stop();

        glib::Continue(true)
    }
}
