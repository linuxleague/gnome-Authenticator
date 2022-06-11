use glib::{clone, Receiver, Sender};
use gtk::{gio, glib, prelude::*, subclass::prelude::*, CompositeTemplate};
use gtk_macros::send;
use tracing::error;

use crate::models::{Provider, FAVICONS_PATH, RUNTIME};

pub enum ImageAction {
    Ready(String),
    Failed,
}

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::{subclass, ParamFlags, ParamSpec, ParamSpecObject, ParamSpecUInt, Value};
    use once_cell::sync::Lazy;

    use super::*;

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
        pub signal_id: RefCell<Option<glib::SignalHandlerId>>,
        pub join_handle: RefCell<Option<tokio::task::JoinHandle<()>>>,
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
                provider: RefCell::default(),
                signal_id: RefCell::default(),
                join_handle: RefCell::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecObject::new(
                        "provider",
                        "",
                        "",
                        Provider::static_type(),
                        ParamFlags::READWRITE,
                    ),
                    ParamSpecUInt::new(
                        "size",
                        "",
                        "",
                        32,
                        96,
                        48,
                        ParamFlags::READWRITE | ParamFlags::CONSTRUCT,
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
    pub fn set_provider(&self, provider: Option<&Provider>) {
        let imp = self.imp();
        if let Some(provider) = provider {
            self.set_property("provider", &provider);
            if provider.website().is_some() || provider.image_uri().is_some() {
                imp.stack.set_visible_child_name("loading");
                imp.spinner.start();
                self.on_provider_image_changed();
            }
            let signal_id = provider.connect_notify_local(
                Some("image-uri"),
                clone!(@weak self as image => move |_, _| {
                    image.on_provider_image_changed();
                }),
            );
            imp.signal_id.replace(Some(signal_id));
            return;
        } else if let (Some(signal_id), Some(provider)) =
            (imp.signal_id.borrow_mut().take(), self.provider())
        {
            provider.disconnect(signal_id);
        }

        imp.image.set_from_icon_name(Some("provider-fallback"));
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
                let small_file = gio::File::for_path(&FAVICONS_PATH.join(format!("{uri}_32x32")));
                let large_file = gio::File::for_path(&FAVICONS_PATH.join(format!("{uri}_96x96")));
                if !small_file.query_exists(gio::Cancellable::NONE)
                    || !large_file.query_exists(gio::Cancellable::NONE)
                {
                    self.fetch();
                    return;
                }
                if imp.size.get() == 32 {
                    imp.image.set_from_file(small_file.path());
                } else {
                    imp.image.set_from_file(large_file.path());
                }
                imp.stack.set_visible_child_name("image");
            }
            _ => {
                self.fetch();
            }
        }
    }

    fn fetch(&self) {
        let imp = self.imp();
        if let Some(handle) = imp.join_handle.borrow_mut().take() {
            handle.abort();
        }
        if let Some(provider) = self.provider() {
            imp.stack.set_visible_child_name("loading");
            imp.spinner.start();

            if let Some(website) = provider.website() {
                let id = provider.id();
                let name = provider.name();
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let future = async move {
                    match Provider::favicon(website, name, id).await {
                        Ok(cache_name) => {
                            sender.send(Some(cache_name)).unwrap();
                        }
                        Err(err) => {
                            tracing::error!("Failed to load favicon {}", err);
                            sender.send(None).unwrap();
                        }
                    };
                };
                let join_handle = RUNTIME.spawn(future);
                imp.join_handle.borrow_mut().replace(join_handle);

                glib::MainContext::default().spawn_local(clone!(@weak self as this => async move {
                   let imp = this.imp();
                    match receiver.await {
                        Ok(Some(cache_name)) => {
                            send!(imp.sender.clone(), ImageAction::Ready(cache_name));
                        }
                        Ok(None) =>  {
                            send!(imp.sender.clone(), ImageAction::Failed);
                        },
                        Err(_) => {
                            tracing::debug!("Provider image fetching aborted");
                        }
                    };
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
            // TODO: handle network failure and other errors differently
            ImageAction::Failed => {
                imp.image.set_from_icon_name(Some("provider-fallback"));
                "invalid".to_string()
            }
            ImageAction::Ready(cache_name) => {
                if imp.size.get() == 32 {
                    imp.image
                        .set_from_file(Some(&FAVICONS_PATH.join(format!("{cache_name}_32x32"))));
                } else {
                    imp.image
                        .set_from_file(Some(&FAVICONS_PATH.join(format!("{cache_name}_96x96"))));
                }
                cache_name
            }
        };
        if let Some(provider) = self.provider() {
            let guard = provider.freeze_notify();
            if let Err(err) = provider.set_image_uri(&image_path) {
                tracing::warn!("Failed to update provider image {}", err);
            }
            drop(guard);
        }

        imp.stack.set_visible_child_name("image");
        imp.spinner.stop();
        glib::Continue(true)
    }
}
