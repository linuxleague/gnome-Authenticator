use std::os::unix::io::AsRawFd;

use gst::prelude::*;
use gtk::{
    gdk,
    glib::{self, clone, Sender},
    graphene,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::widgets::camera::CameraEvent;
static PIPELINE_NAME: Lazy<glib::GString> = Lazy::new(|| glib::GString::from("camera"));
/// Fancy Camera with QR code detection using ZBar
///
/// Pipeline:
///                            queue -- videoconvert -- zbar -- fakesink
///                         /
///     pipewiresrc -- tee
///                         \
///                            queue -- videoflip - glsinkbin
mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct CameraPaintable {
        pub sender: RefCell<Option<Sender<CameraEvent>>>,
        pub pipeline: RefCell<Option<gst::Pipeline>>,
        pub pipewire_element: RefCell<Option<gst::Element>>,
        pub sink_paintable: RefCell<Option<gdk::Paintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraPaintable {
        const NAME: &'static str = "CameraPaintable";
        type Type = super::CameraPaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for CameraPaintable {
        fn dispose(&self) {
            self.obj().close_pipeline();
        }
    }

    impl PaintableImpl for CameraPaintable {
        fn intrinsic_height(&self) -> i32 {
            if let Some(ref paintable) = *self.sink_paintable.borrow() {
                paintable.intrinsic_height()
            } else {
                0
            }
        }

        fn intrinsic_width(&self) -> i32 {
            if let Some(ref paintable) = *self.sink_paintable.borrow() {
                paintable.intrinsic_width()
            } else {
                0
            }
        }

        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(ref image) = *self.sink_paintable.borrow() {
                // Transformation to avoid stretching the camera. We translate and scale the
                // image.
                let aspect = width / height.max(std::f64::EPSILON); // Do not divide by zero.
                let image_aspect = image.intrinsic_aspect_ratio();

                if image_aspect == 0.0 {
                    image.snapshot(snapshot, width, height);
                    return;
                };

                let (new_width, new_height) = match aspect <= image_aspect {
                    true => (height * image_aspect, height), // Mobile view
                    false => (width, width / image_aspect),  // Landscape
                };

                let p = graphene::Point::new(
                    ((width - new_width) / 2.0) as f32,
                    ((height - new_height) / 2.0) as f32,
                );
                snapshot.translate(&p);

                image.snapshot(snapshot, new_width, new_height);
            } else {
                snapshot.append_color(
                    &gdk::RGBA::BLACK,
                    &graphene::Rect::new(0f32, 0f32, width as f32, height as f32),
                );
            }
        }
    }
}

glib::wrapper! {
    pub struct CameraPaintable(ObjectSubclass<imp::CameraPaintable>)
        @implements gdk::Paintable;
}

impl CameraPaintable {
    pub fn new(sender: Sender<CameraEvent>) -> Self {
        let paintable = glib::Object::new::<Self>();
        paintable.imp().sender.replace(Some(sender));
        paintable
    }

    pub fn set_pipewire_node_id(&self, node_id: u32) -> anyhow::Result<()> {
        let pipewire_element = self.imp().pipewire_element.borrow().clone().unwrap();
        pipewire_element.set_property("path", node_id.to_string());
        tracing::debug!("Loading PipeWire Node ID: {node_id}");
        self.close_pipeline();
        self.init_pipeline(&pipewire_element)?;
        Ok(())
    }

    pub fn set_pipewire_fd<F: AsRawFd>(&self, fd: F) -> anyhow::Result<()> {
        let raw_fd = fd.as_raw_fd();
        let pipewire_element = gst::ElementFactory::make_with_name("pipewiresrc", None)?;
        pipewire_element.set_property("fd", raw_fd);
        tracing::debug!("Loading PipeWire with FD: {}", raw_fd);
        self.imp().pipewire_element.replace(Some(pipewire_element));
        Ok(())
    }

    fn init_pipeline(&self, pipewire_src: &gst::Element) -> anyhow::Result<()> {
        tracing::debug!("Init pipeline");
        let imp = self.imp();
        let pipeline = gst::Pipeline::new(None);

        let sink = gst::ElementFactory::make_with_name("gtk4paintablesink", None)?;
        let paintable = sink.property::<gdk::Paintable>("paintable");

        paintable.connect_invalidate_contents(clone!(@weak self as pt => move |_| {
            pt.invalidate_contents();
        }));

        paintable.connect_invalidate_size(clone!(@weak self as pt => move |_| {
            pt.invalidate_size();
        }));
        let tee = gst::ElementFactory::make_with_name("tee", None)?;
        let videoconvert = gst::ElementFactory::make_with_name("videoconvert", None)?;
        let queue1 = gst::ElementFactory::make_with_name("queue", None)?;
        let queue2 = gst::ElementFactory::make_with_name("queue", None)?;
        let zbar = gst::ElementFactory::make_with_name("zbar", None)?;
        let fakesink = gst::ElementFactory::make_with_name("fakesink", None)?;

        let videoflip = gst::ElementFactory::make("videoflip")
            .property("video-direction", gst_video::VideoOrientationMethod::Auto)
            .build()?;

        let sink = if paintable
            .property::<Option<gdk::GLContext>>("gl-context")
            .is_some()
        {
            gst::ElementFactory::make("glsinkbin")
                .property("sink", &sink)
                .build()?
        } else {
            let bin = gst::Bin::default();
            let convert = gst::ElementFactory::make_with_name("videoconvert", None)?;

            bin.add(&convert)?;
            bin.add(&sink)?;
            convert.link(&sink)?;

            bin.add_pad(&gst::GhostPad::with_target(
                Some("sink"),
                &convert.static_pad("sink").unwrap(),
            )?)?;

            bin.upcast()
        };
        imp.sink_paintable.replace(Some(paintable));

        pipeline.add_many(&[
            pipewire_src,
            &tee,
            &queue1,
            &videoconvert,
            &zbar,
            &fakesink,
            &queue2,
            &videoflip,
            &sink,
        ])?;

        gst::Element::link_many(&[pipewire_src, &tee, &queue1, &videoconvert, &zbar, &fakesink])?;
        tee.link_pads(None, &queue2, None)?;
        gst::Element::link_many(&[&queue2, &videoflip, &sink])?;

        let bus = pipeline.bus().unwrap();
        bus.add_watch_local(
            clone!(@weak self as paintable => @default-return glib::Continue(false), move |_, msg| {
                use gst::MessageView;
                let sender = paintable.imp().sender.borrow().as_ref().unwrap().clone();
                match msg.view() {
                    MessageView::Error(err) => {
                        tracing::error!(
                            "Error from {:?}: {} ({:?})",
                            err.src().map(|s| s.path_string()),
                            err.error(),
                            err.debug()
                        );
                    }
                    MessageView::StateChanged(state) => {
                        if Some(&*PIPELINE_NAME) == state.src().map(|s| s.name()).as_ref() {
                            let structure = state.structure().unwrap();
                            let new_state = structure.get::<gst::State>("new-state")
                                .unwrap();
                            if new_state == gst::State::Playing {
                                sender.send(CameraEvent::StreamStarted).unwrap();
                            }
                        }
                    }
                    MessageView::Element(e) => {
                        if let Some(s) = e.structure() {
                            if let Ok(symbol) = s.get::<String>("symbol") {
                               sender.send(CameraEvent::CodeDetected(symbol)).unwrap();
                            }
                        }
                    }
                    _ => (),
                }
                glib::Continue(true)

            }),
        )?;
        imp.pipeline.replace(Some(pipeline));
        Ok(())
    }

    pub fn close_pipeline(&self) {
        tracing::debug!("Closing pipeline");
        if let Some(pipeline) = self.imp().pipeline.borrow_mut().take() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                tracing::error!("Failed to close the pipeline: {err}");
            }
        }
    }

    pub fn start(&self) {
        if let Some(pipeline) = &*self.imp().pipeline.borrow() {
            if let Err(err) = pipeline.set_state(gst::State::Playing) {
                tracing::error!("Failed to start the camera stream: {err}");
            }
        }
    }

    pub fn stop(&self) {
        if let Some(pipeline) = &*self.imp().pipeline.borrow() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                tracing::error!("Failed to stop the camera stream: {err}");
            }
        }
    }
}
