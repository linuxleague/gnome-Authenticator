use std::os::unix::io::AsRawFd;

use crate::widgets::camera::CameraEvent;
use gst::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{
    gdk,
    glib::{self, clone, Sender},
    graphene,
};
use gtk_macros::send;
use once_cell::sync::Lazy;
static PIPELINE_NAME: Lazy<glib::GString> = Lazy::new(|| glib::GString::from("camera"));
/// Fancy Camera with QR code detection using ZBar
///
/// Pipeline:
///                            queue -- videoconvert -- zbar -- fakesink
///                         /
///     pipewiresrc -- tee
///                         \
///                            queue -- glsinkbin
///
///
mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default)]
    pub struct CameraPaintable {
        pub sender: RefCell<Option<Sender<CameraEvent>>>,
        pub pipeline: RefCell<Option<gst::Pipeline>>,
        pub sink_paintable: RefCell<Option<gdk::Paintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraPaintable {
        const NAME: &'static str = "CameraPaintable";
        type Type = super::CameraPaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for CameraPaintable {
        fn dispose(&self, paintable: &Self::Type) {
            paintable.close_pipeline();
        }
    }

    impl PaintableImpl for CameraPaintable {
        fn intrinsic_height(&self, _paintable: &Self::Type) -> i32 {
            if let Some(ref paintable) = *self.sink_paintable.borrow() {
                paintable.intrinsic_height()
            } else {
                0
            }
        }

        fn intrinsic_width(&self, _paintable: &Self::Type) -> i32 {
            if let Some(ref paintable) = *self.sink_paintable.borrow() {
                paintable.intrinsic_width()
            } else {
                0
            }
        }

        fn snapshot(
            &self,
            _paintable: &Self::Type,
            snapshot: &gdk::Snapshot,
            width: f64,
            height: f64,
        ) {
            let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();
            if let Some(ref image) = *self.sink_paintable.borrow() {
                // Transformation to avoid stretching the camera. We translate and scale the image.
                let aspect = width / height.max(std::f64::EPSILON); // Do not divide by zero.
                let image_aspect = image.intrinsic_aspect_ratio();

                if image_aspect == 0.0 {
                    image.snapshot(snapshot.upcast_ref(), width, height);
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

                image.snapshot(snapshot.upcast_ref(), new_width, new_height);
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
    pub struct CameraPaintable(ObjectSubclass<imp::CameraPaintable>) @implements gdk::Paintable;
}

impl CameraPaintable {
    pub fn new(sender: Sender<CameraEvent>) -> Self {
        let paintable = glib::Object::new::<Self>(&[]).expect("Failed to create a CameraPaintable");
        paintable.imp().sender.replace(Some(sender));
        paintable
    }

    pub fn set_pipewire_node_id<F: AsRawFd>(&self, fd: F, node_id: u32) {
        let raw_fd = fd.as_raw_fd();
        log::debug!("Loading PipeWire Node ID: {} with FD: {}", node_id, raw_fd);
        let pipewire_element = gst::ElementFactory::make("pipewiresrc", None).unwrap();
        pipewire_element.set_property("fd", &raw_fd);
        pipewire_element.set_property("path", &node_id.to_string());
        self.init_pipeline(pipewire_element);
    }

    fn init_pipeline(&self, pipewire_src: gst::Element) {
        log::debug!("Init pipeline");
        let imp = self.imp();
        let pipeline = gst::Pipeline::new(None);

        let sink = gst::ElementFactory::make("gtk4paintablesink", None).unwrap();
        let paintable = sink.property::<gdk::Paintable>("paintable");

        paintable.connect_invalidate_contents(clone!(@weak self as pt => move |_| {
            pt.invalidate_contents();
        }));

        paintable.connect_invalidate_size(clone!(@weak self as pt => move |_| {
            pt.invalidate_size  ();
        }));
        imp.sink_paintable.replace(Some(paintable));
        let tee = gst::ElementFactory::make("tee", None).unwrap();
        let videoconvert1 = gst::ElementFactory::make("videoconvert", None).unwrap();
        let videoconvert2 = gst::ElementFactory::make("videoconvert", None).unwrap();
        let queue1 = gst::ElementFactory::make("queue", None).unwrap();
        let queue2 = gst::ElementFactory::make("queue", None).unwrap();
        let zbar = gst::ElementFactory::make("zbar", None).unwrap();
        let fakesink = gst::ElementFactory::make("fakesink", None).unwrap();

        pipeline
            .add_many(&[
                &pipewire_src,
                &tee,
                &queue1,
                &videoconvert1,
                &zbar,
                &fakesink,
                &queue2,
                &videoconvert2,
                &sink,
            ])
            .unwrap();

        gst::Element::link_many(&[
            &pipewire_src,
            &tee,
            &queue1,
            &videoconvert1,
            &zbar,
            &fakesink,
        ])
        .unwrap();
        tee.link_pads(None, &queue2, None).unwrap();
        gst::Element::link_many(&[&queue2, &videoconvert2, &sink]).unwrap();

        let bus = pipeline.bus().unwrap();
        bus.add_watch_local(
            clone!(@weak self as paintable => @default-return glib::Continue(false), move |_, msg| {
                use gst::MessageView;
                let sender = paintable.imp().sender.borrow().as_ref().unwrap().clone();
                match msg.view() {
                    MessageView::Error(err) => {
                        log::error!(
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
                                send!(sender, CameraEvent::StreamStarted);
                            }
                        }
                    }
                    MessageView::Element(e) => {
                        if let Some(s) = e.structure() {
                            if let Ok(symbol) = s.get::<String>("symbol") {
                               send!(sender, CameraEvent::CodeDetected(symbol));
                            }
                        }
                    }
                    _ => (),
                }
                glib::Continue(true)

            }),
        )
        .expect("Failed to add bus watch");
        imp.pipeline.replace(Some(pipeline));
    }

    pub fn close_pipeline(&self) {
        log::debug!("Closing pipeline");
        if let Some(pipeline) = self.imp().pipeline.borrow_mut().take() {
            pipeline.set_state(gst::State::Null).unwrap();
        }
    }

    pub fn start(&self) {
        if let Some(pipeline) = &*self.imp().pipeline.borrow() {
            if let Err(err) = pipeline.set_state(gst::State::Playing) {
                log::error!("Failed to start the camera stream: {}", err);
            }
        }
    }

    pub fn stop(&self) {
        if let Some(pipeline) = &*self.imp().pipeline.borrow() {
            if let Err(err) = pipeline.set_state(gst::State::Null) {
                log::error!("Failed to stop the camera stream: {}", err);
            }
        }
    }
}
