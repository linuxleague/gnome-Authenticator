use crate::models::Provider;
use crate::models::RUNTIME;
use glib::{clone, Receiver, Sender};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::send;

pub enum ImageAction {
    Ready(gio::File),
    Failed,
}

mod imp {
    use super::*;
    use glib::{subclass, ParamSpec, ParamSpecObject, ParamSpecUInt, Value};
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

    #[glib::object_subclass]
    impl ObjectSubclass for ProviderImage {
        const NAME: &'static str = "ProviderImage";
        type Type = super::ProviderImage;
        type ParentType = gtk::Box;

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

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
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
                    ParamSpecObject::new(
                        "provider",
                        "provider",
                        "Provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpecUInt::new(
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
        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                "size" => {
                    let size = value.get().unwrap();
                    self.size.set(size);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
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
        let imp = self.imp();
        if let Some(provider) = provider {
            imp.stack.set_visible_child_name("loading");
            imp.spinner.start();
            self.set_property("provider", &provider);
            self.on_provider_image_changed();
            provider.connect_notify_local(
                Some("image-uri"),
                clone!(@weak self as image => move |_, _| {
                    image.on_provider_image_changed();
                }),
            );
        } else {
            imp.image.set_from_icon_name(Some("provider-fallback"));
        }
    }

    fn on_provider_image_changed(&self) {
        let imp = self.imp();
        let provider = self.provider().unwrap();
        match provider.image_uri() {
            Some(uri) => {
                // Very dirty hack to store that we couldn't find an icon
                // to avoid re-hitting the website every time we have to display it
                if uri == "invalid" {
                    imp.image.set_from_icon_name(Some("provider-fallback"));
                    imp.stack.set_visible_child_name("image");
                    return;
                }

                let file = gio::File::for_uri(&uri);
                if !file.query_exists(gio::Cancellable::NONE) {
                    self.fetch();
                    return;
                }

                imp.image.set_from_file(file.path());
                imp.stack.set_visible_child_name("image");
            }
            _ => {
                self.fetch();
            }
        }
    }

    fn fetch(&self) {
        let imp = self.imp();
        if let Some(provider) = self.provider() {
            imp.stack.set_visible_child_name("loading");
            imp.spinner.start();

            if let Some(website) = provider.website() {
                let id = provider.id();
                let name = provider.name();
                let (sender, receiver) = futures::channel::oneshot::channel();
                RUNTIME.spawn(async move {
                    match Provider::favicon(website, name, id).await {
                        Ok(file) => {
                            sender.send(Some(file)).unwrap();
                        }
                        Err(err) => {
                            log::error!("Failed to load favicon {}", err);
                            sender.send(None).unwrap();
                        }
                    }
                });
                glib::MainContext::default().spawn_local(clone!(@weak self as this => async move {
                   let imp = this.imp();
                    let response = receiver.await.unwrap();
                    if let Some(file) = response {
                        send!(imp.sender.clone(), ImageAction::Ready(file));
                    } else {
                        send!(imp.sender.clone(), ImageAction::Failed);
                    }
                }));
            }
        }
    }

    pub fn reset(&self) {
        self.imp()
            .image
            .set_from_icon_name(Some("provider-fallback"));
        self.fetch();
    }

    pub fn set_from_file(&self, file: &gio::File) {
        let imp = self.imp();

        imp.image.set_from_file(file.path());
        imp.stack.set_visible_child_name("image");
    }

    fn provider(&self) -> Option<Provider> {
        self.property("provider")
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        let receiver = imp.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as image => @default-return glib::Continue(false), move |action| image.do_action(action)),
        );
        self.bind_property("size", &*imp.image, "pixel-size")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    fn do_action(&self, action: ImageAction) -> glib::Continue {
        let imp = self.imp();
        let image_path = match action {
            //TODO: handle network failure and other errors differently
            ImageAction::Failed => {
                imp.image.set_from_icon_name(Some("provider-fallback"));
                "invalid".to_string()
            }
            ImageAction::Ready(image) => {
                imp.image.set_from_file(image.path());
                let image_uri = image.uri();
                image_uri.to_string()
            }
        };
        if let Some(provider) = self.provider() {
            if let Err(err) = provider.set_image_uri(&image_path) {
                warn!("Failed to update provider image {}", err);
            }
        }

        imp.stack.set_visible_child_name("image");
        imp.spinner.stop();
        glib::Continue(true)
    }
}
