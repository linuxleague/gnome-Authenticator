use crate::widgets::CameraPaintable;
use gst::prelude::*;
use gtk::{
    glib::{self, clone, Receiver},
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
use gtk_macros::spawn;
use once_cell::sync::Lazy;
use std::os::unix::prelude::RawFd;

mod screenshot {
    use super::*;
    use anyhow::Result;
    use ashpd::{desktop::screenshot::ScreenshotProxy, zbus, WindowIdentifier};
    use gtk::gio;
    use image::GenericImageView;
    use zbar_rust::ZBarImageScanner;

    pub async fn scan(data: &[u8]) -> Result<String> {
        // remove the file after reading the data
        let img = image::load_from_memory(&data)?;

        let (width, height) = img.dimensions();
        let img_data: Vec<u8> = img.to_luma8().to_vec();

        let mut scanner = ZBarImageScanner::new();

        let results = scanner
            .scan_y800(&img_data, width, height)
            .map_err(|e| anyhow::format_err!(e))?;

        if let Some(result) = results.get(0) {
            let content = String::from_utf8(result.data.clone())?;
            return Ok(content);
        }
        anyhow::bail!("Invalid QR code")
    }

    pub async fn capture(window: gtk::Window) -> Result<gio::File> {
        let connection = zbus::Connection::session().await?;
        let proxy = ScreenshotProxy::new(&connection).await?;
        let uri = proxy
            .screenshot(&WindowIdentifier::from_native(&window).await, true, true)
            .await?;
        Ok(gio::File::for_uri(&uri))
    }

    pub async fn stream() -> Result<Option<(RawFd, Option<u32>)>> {
        let connection = zbus::Connection::session().await?;
        let proxy = ashpd::desktop::camera::CameraProxy::new(&connection).await?;
        if !proxy.is_camera_present().await? {
            return Ok(None);
        }
        proxy.access_camera().await?;

        let stream_fd = proxy.open_pipe_wire_remote().await?;
        let node_id = ashpd::desktop::camera::pipewire_node_id(stream_fd).await?;
        Ok(Some((stream_fd, node_id)))
    }
}

#[derive(Debug)]
pub enum CameraEvent {
    CodeDetected(String),
    StreamStarted,
}

#[derive(Debug)]
pub enum CameraState {
    NotFound,
    Ready,
}

mod imp {
    use super::*;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/camera.ui")]
    pub struct Camera {
        pub paintable: CameraPaintable,
        pub receiver: RefCell<Option<Receiver<CameraEvent>>>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));

            Self {
                paintable: CameraPaintable::new(sender),
                receiver,
                spinner: TemplateChild::default(),
                stack: TemplateChild::default(),
                picture: TemplateChild::default(),
            }
        }
    }

    impl ObjectImpl for Camera {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "code-detected",
                    &[String::static_type().into()],
                    <()>::static_type().into(),
                )
                .run_first()
                .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.init_widgets();
            self.parent_constructed(obj);
        }
        fn dispose(&self, _obj: &Self::Type) {
            self.paintable.close_pipeline();
        }
    }
    impl WidgetImpl for Camera {}
    impl adw::subclass::prelude::BinImpl for Camera {}
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>) @extends gtk::Widget, adw::Bin;
}

impl Camera {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a Camera")
    }

    fn set_state(&self, state: CameraState) {
        let imp = self.imp();
        tracing::info!("The camera state changed to {:#?}", state);
        match state {
            CameraState::NotFound => {
                imp.stack.set_visible_child_name("not-found");
            }
            CameraState::Ready => {
                imp.stack.set_visible_child_name("stream");
                imp.spinner.stop();
            }
        }
    }

    fn do_event(&self, event: CameraEvent) -> glib::Continue {
        match event {
            CameraEvent::CodeDetected(code) => {
                self.emit_by_name::<()>("code-detected", &[&code]);
            }
            CameraEvent::StreamStarted => {
                self.set_state(CameraState::Ready);
            }
        }

        glib::Continue(true)
    }

    pub fn start(&self) {
        self.imp().paintable.start();
        self.set_state(CameraState::Ready);
    }

    pub fn stop(&self) {
        self.imp().paintable.stop();
    }

    pub fn from_camera(&self) {
        spawn!(clone!(@weak self as camera => async move {
            match screenshot::stream().await {
                Ok(Some((stream_fd, node_id))) => {
                    match camera.imp().paintable.set_pipewire_node_id(stream_fd, node_id) {
                        Ok(_) => camera.start(),
                        Err(err) => tracing::error!("Failed to start the camera stream {err}"),
                    };
                },
                Ok(None) => {
                    camera.set_state(CameraState::NotFound);
                }
                Err(e) => tracing::error!("Failed to stream {}", e),
            }
        }));
    }

    pub async fn from_screenshot(&self) -> anyhow::Result<()> {
        let window = self.root().unwrap().downcast::<gtk::Window>().unwrap();
        let screenshot_file = screenshot::capture(window).await?;
        let (data, _) = screenshot_file.load_contents_future().await?;
        if let Ok(code) = screenshot::scan(&data).await {
            self.emit_by_name::<()>("code-detected", &[&code]);
        }
        if let Err(err) = screenshot_file
            .trash_future(glib::source::PRIORITY_HIGH)
            .await
        {
            tracing::error!("Failed to remove scanned screenshot {}", err);
        }
        Ok(())
    }

    fn init_widgets(&self) {
        let imp = self.imp();
        self.set_state(CameraState::NotFound);
        let receiver = imp.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            glib::clone!(@weak self as camera => @default-return glib::Continue(false), move |action| camera.do_event(action)),
        );
        imp.picture.set_paintable(Some(&imp.paintable));
    }
}
