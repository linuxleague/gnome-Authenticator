use std::{
    cell::{Cell, RefCell},
    os::unix::prelude::RawFd,
};

use adw::subclass::prelude::*;
use anyhow::Result;
use ashpd::{desktop::screenshot::ScreenshotProxy, zbus};
use gst::prelude::*;
use gtk::{
    gio,
    glib::{
        self, clone,
        subclass::{InitializingObject, Signal},
        Receiver,
    },
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
use gtk_macros::spawn;
use image::GenericImageView;
use once_cell::sync::Lazy;
use zbar_rust::ZBarImageScanner;

use crate::widgets::CameraPaintable;

mod screenshot {
    use super::*;

    pub fn scan(data: &[u8]) -> Result<String> {
        // remove the file after reading the data
        let img = image::load_from_memory(data)?;

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

    pub async fn capture(window: Option<gtk::Window>) -> Result<gio::File> {
        let connection = zbus::Connection::session().await?;
        let proxy = ScreenshotProxy::new(&connection).await?;
        let uri = proxy
            .screenshot(
                &{
                    if let Some(ref window) = window {
                        ashpd::WindowIdentifier::from_native(window).await
                    } else {
                        ashpd::WindowIdentifier::default()
                    }
                },
                true,
                true,
            )
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

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/camera.ui")]
    pub struct Camera {
        pub paintable: CameraPaintable,
        pub receiver: RefCell<Option<Receiver<CameraEvent>>>,
        pub started: Cell<bool>,
        #[template_child]
        pub previous: TemplateChild<gtk::Button>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub screenshot: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));

            Self {
                paintable: CameraPaintable::new(sender),
                receiver,
                started: Cell::default(),
                previous: TemplateChild::default(),
                spinner: TemplateChild::default(),
                stack: TemplateChild::default(),
                picture: TemplateChild::default(),
                screenshot: TemplateChild::default(),
            }
        }
    }

    impl ObjectImpl for Camera {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("close", &[], <()>::static_type().into())
                        .action()
                        .build(),
                    Signal::builder(
                        "code-detected",
                        &[String::static_type().into()],
                        <()>::static_type().into(),
                    )
                    .run_first()
                    .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.setup_receiver();
            obj.setup_widget();
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.paintable.close_pipeline();
        }
    }

    impl WidgetImpl for Camera {}
    impl BinImpl for Camera {}
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>)
        @extends gtk::Widget, adw::Bin;
}

impl Camera {
    pub fn start(&self) {
        let imp = self.imp();
        if !imp.started.get() {
            imp.paintable.start();
            imp.started.set(true);
            self.set_state(CameraState::Ready);
        }
    }

    pub fn stop(&self) {
        let imp = self.imp();
        if imp.started.get() {
            imp.paintable.stop();
            imp.started.set(false);
        }
    }

    pub fn scan_from_camera(&self) {
        if !self.imp().started.get() {
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
    }

    pub async fn scan_from_screenshot(&self) -> anyhow::Result<()> {
        let screenshot_file = screenshot::capture(
            self.root()
                .map(|root| root.downcast::<gtk::Window>().unwrap()),
        )
        .await?;
        let (data, _) = screenshot_file.load_contents_future().await?;
        if let Ok(code) = screenshot::scan(&data) {
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

    fn setup_receiver(&self) {
        self.imp().receiver.borrow_mut().take().unwrap().attach(
            None,
            glib::clone!(@weak self as camera => @default-return glib::Continue(false), move |event| {
                match event {
                    CameraEvent::CodeDetected(code) => {
                        camera.emit_by_name::<()>("code-detected", &[&code]);
                    }
                    CameraEvent::StreamStarted => {
                        camera.set_state(CameraState::Ready);
                    }
                }
                glib::Continue(true)
            }),
        );
    }

    fn setup_widget(&self) {
        let imp = self.imp();
        self.set_state(CameraState::NotFound);
        imp.picture.set_paintable(Some(&imp.paintable));

        imp.previous
            .connect_clicked(clone!(@weak self as camera => move |_| {
                camera.emit_by_name::<()>("close", &[]);
            }));

        imp.screenshot
            .connect_clicked(clone!(@weak self as camera => move |_| {
                spawn!(clone!(@strong camera => async move {
                    // TODO: Error handling?
                    let _ = camera.scan_from_screenshot().await;
                }));
            }));
    }
}

impl Default for Camera {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create a Camera")
    }
}
