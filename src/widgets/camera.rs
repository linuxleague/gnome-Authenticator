use glib::{Receiver, Sender};
use gst::prelude::*;
use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
use gtk_macros::send;
use once_cell::sync::Lazy;
/// Fancy Camera with QR code detection using ZBar
///
/// Pipeline:
///                            queue -- videoconvert -- zbar -- fakesink
///                         /
///     device sink -- tee
///                         \
///                            queue -- glsinkbin
///
///

static PIPELINE_NAME: Lazy<glib::GString> = Lazy::new(|| glib::GString::from("camera"));

mod screenshot {
    use anyhow::Result;
    use ashpd::{
        desktop::screenshot::{Screenshot, ScreenshotOptions, ScreenshotProxy},
        zbus, RequestProxy, Response, WindowIdentifier,
    };
    use gtk::{gio, prelude::*};
    use image::GenericImageView;
    use zbar_rust::ZBarImageScanner;

    pub fn scan(screenshot: &gio::File) -> Result<String> {
        let (data, _) = screenshot.load_contents(gio::NONE_CANCELLABLE)?;

        let img = image::load_from_memory(&data)?;

        let (width, height) = img.dimensions();
        let img_data: Vec<u8> = img.to_luma8().to_vec();

        let mut scanner = ZBarImageScanner::new();

        let results = scanner
            .scan_y800(&img_data, width, height)
            .map_err(|e| anyhow::format_err!(e))?;

        if let Some(ref result) = results.get(0) {
            let content = String::from_utf8(result.data.clone())?;
            return Ok(content);
        }
        anyhow::bail!("Invalid QR code")
    }

    pub fn capture<F: FnOnce(gio::File)>(window: gtk::Window, callback: F) -> Result<()> {
        let connection = zbus::Connection::new_session()?;
        let proxy = ScreenshotProxy::new(&connection)?;
        let handle = proxy.screenshot(
            WindowIdentifier::from(window),
            ScreenshotOptions::default().interactive(true).modal(true),
        )?;
        let request = RequestProxy::new(&connection, &handle)?;
        request.on_response(move |response: Response<Screenshot>| {
            if let Ok(screenshot) = response {
                callback(gio::File::new_for_uri(&screenshot.uri));
            }
        })?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum CameraEvent {
    CodeDetected(String),
    DeviceAdded(gst::Device),
    DeviceSelected(gst::Device),
    DeviceRemoved(gst::Device),
    StreamStarted,
}

#[derive(Debug)]
pub enum CameraState {
    Loading,
    NotFound,
    Ready,
    Paused,
}

mod imp {
    use super::*;
    use glib::subclass::{self, Signal};
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/camera.ui")]
    pub struct Camera {
        pub sender: Sender<CameraEvent>,
        pub receiver: RefCell<Option<Receiver<CameraEvent>>>,
        pub pipeline: gst::Pipeline,
        pub sink: gst::Element,
        pub selected_device: RefCell<Option<gst::Device>>,
        pub devices: gio::ListStore,
        pub monitor: gst::DeviceMonitor,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            let pipeline = gst::Pipeline::new(Some(&*PIPELINE_NAME));
            let sink = gst::ElementFactory::make("gtk4glsink", None).unwrap();
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));

            Self {
                sink,
                sender,
                receiver,
                pipeline,
                selected_device: RefCell::default(),
                spinner: TemplateChild::default(),
                stack: TemplateChild::default(),
                overlay: TemplateChild::default(),
                monitor: gst::DeviceMonitor::new(),
                devices: gio::ListStore::new(gst::Device::static_type()),
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
            obj.init_monitor();
            self.parent_constructed(obj);
        }
        fn dispose(&self, _obj: &Self::Type) {
            self.monitor.stop();
            self.pipeline.set_state(gst::State::Null).unwrap();
        }
    }
    impl WidgetImpl for Camera {}
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>) @extends gtk::Widget;
}

impl Camera {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a Camera")
    }

    fn init_monitor(&self) {
        let self_ = imp::Camera::from_instance(self);
        let caps = gst::Caps::new_simple("video/x-raw", &[]);
        self_.monitor.add_filter(Some("Video/Source"), Some(&caps));

        self_.monitor.start().unwrap();
        let bus = self_.monitor.get_bus();
        bus.add_watch_local(clone!(@strong self_.sender as sender => move |_, msg| {
                use gst::MessageView;
                match msg.view() {
                    MessageView::DeviceAdded(event) => {
                        let device = event.get_device();
                        send!(sender, CameraEvent::DeviceAdded(device));
                    }
                    MessageView::DeviceRemoved(event) => {
                        let device = event.get_device();
                        send!(sender, CameraEvent::DeviceRemoved(device));
                    }
                    _ => (),
                };

            glib::Continue(true)
        }))
        .expect("Failed to attach a monitor");
    }

    fn init_pipelines(&self, source_element: gst::Element) {
        let self_ = imp::Camera::from_instance(self);

        let tee = gst::ElementFactory::make("tee", None).unwrap();
        let queue = gst::ElementFactory::make("queue", None).unwrap();
        let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
        let zbar = gst::ElementFactory::make("zbar", None).unwrap();
        let fakesink = gst::ElementFactory::make("fakesink", None).unwrap();
        let queue2 = gst::ElementFactory::make("queue", None).unwrap();
        let glsinkbin = gst::ElementFactory::make("glsinkbin", None).unwrap();
        glsinkbin.set_property("sink", &self_.sink).unwrap();

        self_
            .pipeline
            .add_many(&[
                &source_element,
                &tee,
                &queue,
                &videoconvert,
                &zbar,
                &fakesink,
                &queue2,
                &glsinkbin,
            ])
            .unwrap();

        gst::Element::link_many(&[
            &source_element,
            &tee,
            &queue,
            &videoconvert,
            &zbar,
            &fakesink,
        ])
        .unwrap();
        tee.link_pads(None, &queue2, None).unwrap();
        gst::Element::link_many(&[&queue2, &glsinkbin]).unwrap();

        let bus = self_.pipeline.get_bus().unwrap();
        bus.add_watch_local(clone!(@strong self_.sender as sender => move |_, msg| {
            use gst::MessageView;
            match msg.view() {
                MessageView::StateChanged(state) => {
                    if Some(&*PIPELINE_NAME) == state.get_src().map(|s| s.get_name()).as_ref() {
                        let structure = state.get_structure().unwrap();
                        let new_state = structure.get::<gst::State>("new-state")
                            .unwrap().unwrap();
                        if new_state == gst::State::Playing {
                            send!(sender, CameraEvent::StreamStarted);
                        }
                    }
                }
                MessageView::Element(e) => {
                    if let Some(s) = e.get_structure() {
                        if let Ok(Some(symbol)) = s.get::<String>("symbol") {
                           send!(sender, CameraEvent::CodeDetected(symbol));
                        }
                    }
                }
                MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.get_src().map(|s| s.get_path_string()),
                        err.get_error(),
                        err.get_debug()
                    );
                }
                _ => (),
            };

            glib::Continue(true)
        }))
        .expect("Failed to add bus watch");
    }

    fn set_state(&self, state: CameraState) {
        let self_ = imp::Camera::from_instance(self);
        info!("The camera state changed to {:#?}", state);
        match state {
            CameraState::NotFound => {
                self_.stack.get().set_visible_child_name("not-found");
            }
            CameraState::Ready => {
                self_.stack.get().set_visible_child_name("stream");
                self_.spinner.get().stop();
            }
            CameraState::Loading => {
                self_.stack.get().set_visible_child_name("loading");
                self_.spinner.get().start();
            }
            CameraState::Paused => {}
        }
    }

    fn do_event(&self, event: CameraEvent) -> glib::Continue {
        let self_ = imp::Camera::from_instance(self);
        match event {
            CameraEvent::CodeDetected(code) => {
                self.emit_by_name("code-detected", &[&code]).unwrap();
            }
            CameraEvent::DeviceAdded(device) => {
                info!("Camera source added: {}", device.get_display_name());
                self_.devices.append(&device);
                if self_.selected_device.borrow_mut().is_none() {
                    send!(self_.sender, CameraEvent::DeviceSelected(device));
                }
            }
            CameraEvent::DeviceSelected(device) => {
                info!("Camera source selected: {}", device.get_display_name());
                // TODO: allow selecting a device and update the sink on the pipeline
                self.set_state(CameraState::Loading);
                let element = device.create_element(None).unwrap();
                self.init_pipelines(element);
                self_.selected_device.replace(Some(device));
            }
            CameraEvent::DeviceRemoved(device) => {
                info!("Camera source removed: {}", device.get_display_name());
                self_.devices.append(&device);
            }
            CameraEvent::StreamStarted => {
                self.set_state(CameraState::Ready);
            }
        }

        glib::Continue(true)
    }

    pub fn start(&self) {
        let self_ = imp::Camera::from_instance(self);
        self_.pipeline.set_state(gst::State::Playing).unwrap();
    }

    pub fn stop(&self) {
        let self_ = imp::Camera::from_instance(self);
        self.set_state(CameraState::Paused);
        self_.pipeline.set_state(gst::State::Null).unwrap();
    }

    pub fn from_screenshot(&self) {
        let self_ = imp::Camera::from_instance(self);
        let window = self.get_root().unwrap().downcast::<gtk::Window>().unwrap();
        screenshot::capture(
            window,
            clone!(@strong self_.sender as sender => move |file| {
                if let Ok(code) = screenshot::scan(&file) {
                    send!(sender, CameraEvent::CodeDetected(code));
                }
            }),
        )
        .ok();
    }

    fn init_widgets(&self) {
        let self_ = imp::Camera::from_instance(self);
        self.set_state(CameraState::NotFound);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as camera => move |action| camera.do_event(action)),
        );

        let widget = self_
            .sink
            .get_property("widget")
            .unwrap()
            .get::<gtk::Widget>()
            .unwrap()
            .unwrap();
        widget.set_property("force-aspect-ratio", &false).unwrap();
        self_.overlay.get().set_child(Some(&widget));
    }
}
