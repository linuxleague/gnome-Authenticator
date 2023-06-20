use std::cell::RefCell;

use adw::subclass::prelude::*;
use anyhow::Result;
use ashpd::desktop::screenshot::ScreenshotRequest;
use gettextrs::gettext;
use gst::prelude::*;
use gtk::{
    gio,
    glib::{self, clone, Receiver},
    prelude::*,
};
use image::GenericImageView;
use once_cell::sync::Lazy;

use super::{CameraItem, CameraRow};
use crate::{utils::spawn_tokio, widgets::CameraPaintable};

static CAMERA_LOCATION: &str = "api.libcamera.location";

pub mod screenshot {
    use super::*;

    pub fn scan(data: &[u8]) -> Result<String> {
        // remove the file after reading the data
        let img = image::load_from_memory(data)?;

        let (width, height) = img.dimensions();
        let img_data: Vec<u8> = img.to_luma8().to_vec();

        let mut scanner = zbar_rust::ZBarImageScanner::new();

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
        let identifier = if let Some(ref window) = window {
            ashpd::WindowIdentifier::from_native(window).await
        } else {
            ashpd::WindowIdentifier::default()
        };
        let uri = spawn_tokio(async {
            ScreenshotRequest::default()
                .identifier(identifier)
                .modal(true)
                .interactive(true)
                .send()
                .await?
                .response()
        })
        .await?;

        Ok(gio::File::for_uri(uri.uri().as_str()))
    }
}

pub enum CameraEvent {
    CodeDetected(String),
    StreamStarted,
}

pub enum CameraState {
    NotFound,
    Ready,
}

mod imp {
    use glib::subclass::{InitializingObject, Signal};

    use super::*;

    #[derive(gtk::CompositeTemplate)]
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
        #[template_child]
        pub screenshot: TemplateChild<gtk::Button>,
        #[template_child]
        pub camera_selection_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub toolbar_view: TemplateChild<adw::ToolbarView>,
        pub stream_list: gio::ListStore,
        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("camera");
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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
                camera_selection_button: TemplateChild::default(),
                spinner: TemplateChild::default(),
                stack: TemplateChild::default(),
                picture: TemplateChild::default(),
                screenshot: TemplateChild::default(),
                toolbar_view: TemplateChild::default(),
                stream_list: gio::ListStore::new(glib::BoxedAnyObject::static_type()),
                selection: Default::default(),
            }
        }
    }

    impl ObjectImpl for Camera {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("close").action().build(),
                    Signal::builder("code-detected")
                        .param_types([String::static_type()])
                        .run_first()
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_receiver();
            obj.setup_widget();
            obj.set_state(CameraState::NotFound);
            self.picture.set_paintable(Some(&self.paintable));
        }

        fn dispose(&self) {
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

#[gtk::template_callbacks]
impl Camera {
    pub fn start(&self) {
        let imp = self.imp();
        imp.paintable.start();
        self.set_state(CameraState::Ready);
    }

    pub fn stop(&self) {
        let imp = self.imp();
        imp.paintable.stop();
        imp.stream_list.remove_all();
        imp.selection.set_selected(gtk::INVALID_LIST_POSITION);
    }

    pub fn connect_close<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_local(
            "close",
            false,
            clone!(@weak self as camera => @default-return None, move |_| {
                callback(&camera);
                None
            }),
        )
    }

    pub fn connect_code_detected<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, String) + 'static,
    {
        self.connect_local(
            "code-detected",
            false,
            clone!(@weak self as camera => @default-return None, move |args| {
                let code = args[1].get::<String>().unwrap();
                callback(&camera, code);
                None
            }),
        )
    }

    fn set_streams(&self, streams: Vec<ashpd::desktop::camera::Stream>) {
        let imp = self.imp();
        let mut selected_stream = 0;
        let mut id = 0;
        for stream in streams {
            let default = gettext("Unknown Device");
            let nick = stream
                .properties()
                .get("node.nick")
                .unwrap_or(&default)
                .to_string();

            if let Some(location) = stream.properties().get(CAMERA_LOCATION) {
                if location == "front" {
                    selected_stream = id;
                }
            }

            let item = CameraItem {
                nick,
                node_id: stream.node_id(),
            };
            imp.stream_list.append(&glib::BoxedAnyObject::new(item));
            id += 1;
        }
        imp.selection.set_selected(selected_stream);
    }

    pub async fn scan_from_camera(&self) {
        match spawn_tokio(ashpd::desktop::camera::request()).await {
            Ok(Some((stream_fd, nodes_id))) => {
                match self.imp().paintable.set_pipewire_fd(stream_fd) {
                    Ok(_) => {
                        self.set_streams(nodes_id);
                    }
                    Err(err) => tracing::error!("Failed to start the camera stream {err}"),
                };
            }
            Ok(None) => {
                self.set_state(CameraState::NotFound);
            }
            Err(e) => tracing::error!("Failed to stream {}", e),
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
        match state {
            CameraState::NotFound => {
                tracing::info!("The camera state changed: Not Found");
                imp.stack.set_visible_child_name("not-found");
                imp.toolbar_view.set_extend_content_to_top_edge(false);
                imp.toolbar_view.remove_css_class("extended");
            }
            CameraState::Ready => {
                tracing::info!("The camera state changed: Ready");
                imp.stack.set_visible_child_name("stream");
                imp.toolbar_view.set_extend_content_to_top_edge(true);
                imp.toolbar_view.add_css_class("extended");
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
        let popover = gtk::Popover::new();
        popover.add_css_class("menu");

        imp.selection.set_model(Some(&imp.stream_list));
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let camera_row = CameraRow::default();

            item.downcast_ref::<gtk::ListItem>()
                .unwrap()
                .set_child(Some(&camera_row));
        });
        let selection = &imp.selection;
        factory.connect_bind(glib::clone!(@weak selection => move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let child = item.child().unwrap();
            let row = child.downcast_ref::<CameraRow>().unwrap();

            let item = item.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
            let camera_item = item.borrow::<CameraItem>();
            row.set_label(&camera_item.nick);

            selection.connect_selected_item_notify(glib::clone!(@weak row, @weak item => move |selection| {
                if let Some(selected_item) = selection.selected_item() {
                    row.set_selected(selected_item == item);
                } else {
                    row.set_selected(false);
                }
            }));
        }));
        let list_view = gtk::ListView::new(Some(imp.selection.clone()), Some(factory));
        popover.set_child(Some(&list_view));

        imp.selection.connect_selected_item_notify(glib::clone!(@weak self as obj, @weak popover => move |selection| {
            if let Some(selected_item) = selection.selected_item() {
                let node_id = selected_item.downcast_ref::<glib::BoxedAnyObject>().unwrap().borrow::<CameraItem>().node_id;
                match obj.imp().paintable.set_pipewire_node_id(node_id) {
                    Ok(_) => {
                        obj.start();
                    },
                    Err(err) => {
                        tracing::error!("Failed to start a camera stream {err}");
                    }
                }
            }
            popover.popdown();
        }));

        imp.camera_selection_button.set_popover(Some(&popover));
    }

    #[template_callback]
    async fn on_screenshot_clicked(&self, _btn: gtk::Button) {
        if let Err(err) = self.scan_from_screenshot().await {
            tracing::error!("Failed to scan from screenshot {err}");
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        glib::Object::new()
    }
}
