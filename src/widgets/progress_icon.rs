use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};

pub(crate) mod imp {
    use super::*;
    use glib::{ParamSpec, ParamSpecBoolean, ParamSpecFloat, Value};
    use gtk::{graphene, gsk};
    use once_cell::sync::Lazy;
    use std::cell::Cell;

    #[derive(Debug, Default)]
    pub struct ProgressIcon {
        pub progress: Cell<f32>,
        pub inverted: Cell<bool>,
        pub clockwise: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgressIcon {
        const NAME: &'static str = "ProgressIcon";
        type Type = super::ProgressIcon;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                progress: Cell::new(0.0),
                inverted: Cell::new(false),
                clockwise: Cell::new(true),
            }
        }
    }

    impl ObjectImpl for ProgressIcon {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecFloat::new(
                        "progress",
                        "Progress",
                        "Progress of the icon",
                        0.0,
                        1.0,
                        0.0,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    ParamSpecBoolean::new(
                        "inverted",
                        "Inverted",
                        "Invert icon colors",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                    ParamSpecBoolean::new(
                        "clockwise",
                        "Clockwise",
                        "Direction of the icon",
                        false,
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "progress" => obj.progress().to_value(),
                "inverted" => obj.inverted().to_value(),
                "clockwise" => obj.clockwise().to_value(),
                _ => unreachable!(),
            }
        }

        fn set_property(&self, obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "progress" => obj.set_progress(value.get().unwrap()),
                "inverted" => obj.set_inverted(value.get().unwrap()),
                "clockwise" => obj.set_clockwise(value.get().unwrap()),
                _ => unreachable!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_valign(gtk::Align::Center);
        }
    }

    impl WidgetImpl for ProgressIcon {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            let size = widget.size() as f32;
            let radius = size / 2.0;
            let color = widget.style_context().lookup_color("accent_color").unwrap();
            let mut alpha;
            let progress = if widget.clockwise() {
                1.0 - widget.progress()
            } else {
                widget.progress()
            };

            let rect = graphene::Rect::new(0.0, 0.0, size, size);
            let circle = gsk::RoundedRect::from_rect(rect.clone(), radius);
            let center = graphene::Point::new(size / 2.0, size / 2.0);

            if widget.inverted() {
                alpha = 1.0;
            } else {
                alpha = 0.15;
            }
            let color = gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha);
            let color_stop = gsk::ColorStop::new(progress, color);

            if widget.inverted() {
                alpha = 0.15;
            } else {
                alpha = 1.0;
            }
            let color = gdk::RGBA::new(color.red(), color.green(), color.blue(), alpha);
            let color_stop_end = gsk::ColorStop::new(progress, color);

            let rotation = 0.0;
            snapshot.push_rounded_clip(&circle);
            snapshot.append_conic_gradient(&rect, &center, rotation, &[color_stop, color_stop_end]);
            snapshot.pop();
        }

        fn measure(
            &self,
            widget: &Self::Type,
            _orientation: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            (widget.size(), widget.size(), -1, -1)
        }
    }
}

glib::wrapper! {
    /// A widget to display the progress of an operation.
    ///
    /// The [`NotificationExt::progress()`] property of [`ProgressIcon`] is a float between 0.0 and 1.0,
    /// inclusive which denote that an operation has started or finished, respectively.
    ///
    /// **Implements**: [`ProgressIconExt`]
    pub struct ProgressIcon(ObjectSubclass<imp::ProgressIcon>)
        @extends gtk::Widget;
}

impl Default for ProgressIcon {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl ProgressIcon {
    /// Creates a new [`ProgressIcon`].
    pub fn new() -> Self {
        Self::default()
    }

    fn size(&self) -> i32 {
        let width = self.width_request();
        let height = self.width_request();

        std::cmp::max(16, std::cmp::min(width, height))
    }
}

pub trait ProgressIconExt {
    /// Gets the child widget of `self`.
    ///
    /// Returns: the progress of `self`
    fn progress(&self) -> f32;

    /// Sets the progress of `self`. `progress` should be between 0.0 and 1.0, inclusive.
    fn set_progress(&self, progress: f32);

    /// Returns whether `self` is inverted.
    fn inverted(&self) -> bool;

    /// Sets whether `self` is inverted.
    fn set_inverted(&self, inverted: bool);

    /// Returns the completion direction of `self`.
    fn clockwise(&self) -> bool;

    /// Sets the progress display direction of `self`.
    fn set_clockwise(&self, clockwise: bool);

    fn connect_progress_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
    fn connect_inverted_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
    fn connect_clockwise_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId;
}

impl<W: IsA<ProgressIcon>> ProgressIconExt for W {
    fn progress(&self) -> f32 {
        self.as_ref().imp().progress.get()
    }
    fn set_progress(&self, progress: f32) {
        if (progress - self.progress()).abs() < f32::EPSILON {
            return;
        }
        let clamped = progress.clamp(0.0, 1.0);
        self.as_ref().imp().progress.replace(clamped);
        self.as_ref().queue_draw();
        self.notify("progress");
    }

    fn inverted(&self) -> bool {
        self.as_ref().imp().inverted.get()
    }
    fn set_inverted(&self, inverted: bool) {
        if inverted == self.inverted() {
            return;
        }
        self.as_ref().imp().inverted.replace(inverted);
        self.as_ref().queue_draw();
        self.notify("inverted");
    }

    fn clockwise(&self) -> bool {
        self.as_ref().imp().clockwise.get()
    }

    fn set_clockwise(&self, clockwise: bool) {
        if clockwise == self.clockwise() {
            return;
        }
        self.as_ref().imp().clockwise.replace(clockwise);
        self.as_ref().queue_draw();
        self.notify("clockwise");
    }

    fn connect_progress_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("progress"), move |this, _| {
            f(this);
        })
    }
    fn connect_inverted_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("inverted"), move |this, _| {
            f(this);
        })
    }
    fn connect_clockwise_notify<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_notify_local(Some("clockwise"), move |this, _| {
            f(this);
        })
    }
}
