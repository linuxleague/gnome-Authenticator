use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

pub(crate) mod imp {
    use std::cell::Cell;

    use glib::{ParamSpec, ParamSpecFloat, Value};
    use gtk::{graphene, gsk};
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ProgressIcon {
        pub progress: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgressIcon {
        const NAME: &'static str = "ProgressIcon";
        type Type = super::ProgressIcon;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for ProgressIcon {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecFloat::builder("progress")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(0.0)
                    .explicit_notify()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "progress" => self.obj().progress().to_value(),
                _ => unreachable!(),
            }
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "progress" => self.obj().set_progress(value.get().unwrap()),
                _ => unreachable!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_valign(gtk::Align::Center);
        }
    }

    impl WidgetImpl for ProgressIcon {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let size = widget.size() as f32;
            let radius = size / 2.0;
            let progress = 1.0 - widget.progress();
            let color = widget
                .style_context()
                .lookup_color("accent_color")
                .unwrap_or_else(|| gdk::RGBA::new(0.47058824, 0.68235296, 0.92941177, 1.0));

            let rect = graphene::Rect::new(0.0, 0.0, size, size);
            let circle = gsk::RoundedRect::from_rect(rect, radius);
            let center = graphene::Point::new(size / 2.0, size / 2.0);

            let color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 0.15);
            let color_stop = gsk::ColorStop::new(progress, color);

            let color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 1.0);
            let color_stop_end = gsk::ColorStop::new(progress, color);

            snapshot.push_rounded_clip(&circle);
            snapshot.append_conic_gradient(&rect, &center, 0.0, &[color_stop, color_stop_end]);
            snapshot.pop();
        }

        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let size = self.obj().size();
            (size, size, -1, -1)
        }
    }
}

glib::wrapper! {
    pub struct ProgressIcon(ObjectSubclass<imp::ProgressIcon>)
        @extends gtk::Widget;
}

impl ProgressIcon {
    pub fn set_progress(&self, progress: f32) {
        if (progress - self.progress()).abs() < f32::EPSILON {
            return;
        }
        let clamped = progress.clamp(0.0, 1.0);
        self.imp().progress.replace(clamped);
        self.queue_draw();
        self.notify("progress");
    }

    fn size(&self) -> i32 {
        let width = self.width_request();
        let height = self.width_request();

        std::cmp::max(16, std::cmp::min(width, height))
    }

    fn progress(&self) -> f32 {
        self.imp().progress.get()
    }
}
