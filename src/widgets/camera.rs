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
use gtk_macros::spawn;
use image::GenericImageView;
use once_cell::sync::Lazy;

use super::{CameraItem, CameraRow};
use crate::widgets::CameraPaintable;

mod screenshot {
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
        let uri = ScreenshotRequest::default()
            .identifier(identifier)
            .modal(true)
            .interactive(true)
            .build()
            .await?;

        Ok(gio::File::for_uri(uri.as_str()))
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
    use glib::subclass::{InitializingObject, Signal};

    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/camera.ui")]
    pub struct Camera {
        pub paintable: CameraPaintable,
        pub receiver: RefCell<Option<Receiver<CameraEvent>>>,
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
        #[template_child]
        pub camera_selection_button: TemplateChild<gtk::MenuButton>,
        pub stream_list: gio::ListStore,
        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
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
                previous: TemplateChild::default(),
                camera_selection_button: TemplateChild::default(),
                spinner: TemplateChild::default(),
                stack: TemplateChild::default(),
                picture: TemplateChild::default(),
                screenshot: TemplateChild::default(),
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
        for stream in streams {
            let default = gettext("Unknown Device");
            let nick = stream
                .properties()
                .get("node.nick")
                .unwrap_or(&default)
                .to_string();

            let item = CameraItem {
                nick,
                node_id: stream.node_id(),
            };
            imp.stream_list.append(&glib::BoxedAnyObject::new(item));
        }
        imp.selection.set_selected(0);
    }

    pub fn scan_from_camera(&self) {
        spawn!(clone!(@weak self as camera => async move {
            match ashpd::desktop::camera::request().await {
                Ok(Some((stream_fd, nodes_id))) => {
                    match camera.imp().paintable.set_pipewire_fd(stream_fd) {
                        Ok(_) => {
                            camera.set_streams(nodes_id);
                        },
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
            let item = item.clone().downcast::<gtk::ListItem>().unwrap();
            let child = item.child().unwrap();
            let row = child.downcast_ref::<CameraRow>().unwrap();

            let item = item.item().unwrap().downcast::<glib::BoxedAnyObject>().unwrap();
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
        let list_view = gtk::ListView::new(Some(&imp.selection), Some(&factory));
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
    fn on_previous_clicked(&self, _btn: gtk::Button) {
        self.emit_by_name::<()>("close", &[]);
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
        glib::Object::new(&[])
    }
}
