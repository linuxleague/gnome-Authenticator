use gtk::{gdk, glib, graphene, prelude::*, subclass::prelude::*};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub struct QRCodeData {
    pub width: i32,
    pub height: i32,
    pub items: Vec<Vec<bool>>,
}

impl From<&str> for QRCodeData {
    fn from(data: &str) -> Self {
        let code = qrcode::QrCode::new(data.as_bytes()).unwrap();
        let items = code
            .render::<char>()
            .quiet_zone(false)
            .module_dimensions(1, 1)
            .build()
            .split('\n')
            .into_iter()
            .map(|line| {
                line.chars()
                    .into_iter()
                    .map(|c| !c.is_whitespace())
                    .collect::<Vec<bool>>()
            })
            .collect::<Vec<Vec<bool>>>();

        let width = items.get(0).unwrap().len() as i32;
        let height = items.len() as i32;
        Self {
            width,
            height,
            items,
        }
    }
}

mod imp {

    fn snapshot_qrcode(snapshot: &gtk::Snapshot, qrcode: &QRCodeData, width: f64, height: f64) {
        let is_dark_theme = gtk::Settings::default()
            .unwrap()
            .is_gtk_application_prefer_dark_theme();
        let square_height = height as f32 / qrcode.height as f32;
        let square_width = width as f32 / qrcode.width as f32;

        qrcode.items.iter().enumerate().for_each(|(y, line)| {
            line.iter().enumerate().for_each(|(x, is_dark)| {
                let color = if *is_dark {
                    if is_dark_theme {
                        gdk::RGBA::WHITE
                    } else {
                        gdk::RGBA::BLACK
                    }
                } else {
                    gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)
                };
                let position = graphene::Rect::new(
                    (x as f32) * square_width,
                    (y as f32) * square_height,
                    square_width,
                    square_height,
                );

                snapshot.append_color(&color, &position);
            });
        });
    }
    use super::*;
    use std::cell::RefCell;

    #[allow(clippy::upper_case_acronyms)]
    #[derive(Debug, Default)]
    pub struct QRCodePaintable {
        pub qrcode: RefCell<Option<QRCodeData>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QRCodePaintable {
        const NAME: &'static str = "QRCodePaintable";
        type Type = super::QRCodePaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for QRCodePaintable {}
    impl PaintableImpl for QRCodePaintable {
        fn snapshot(
            &self,
            _paintable: &Self::Type,
            snapshot: &gdk::Snapshot,
            width: f64,
            height: f64,
        ) {
            let snapshot = snapshot.downcast_ref::<gtk::Snapshot>().unwrap();

            if let Some(ref qrcode) = *self.qrcode.borrow() {
                snapshot_qrcode(snapshot, qrcode, width, height);
            }
        }
    }
}

glib::wrapper! {
    pub struct QRCodePaintable(ObjectSubclass<imp::QRCodePaintable>) @implements gdk::Paintable;
}

impl QRCodePaintable {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a QRCodePaintable")
    }

    pub fn set_qrcode(&self, qrcode: QRCodeData) {
        self.imp().qrcode.replace(Some(qrcode));
        self.invalidate_contents();
    }
}
