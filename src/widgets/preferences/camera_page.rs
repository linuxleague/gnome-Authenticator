use crate::widgets::Camera;
use anyhow::Result;
use gtk::{
    gio,
    glib::{self, clone, subclass::InitializingObject},
    prelude::*,
    subclass::prelude::*,
    CompositeTemplate,
};
use gtk_macros::get_action;
use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::rc::Rc;
use tokio::sync::oneshot;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences_camera_page.ui")]
    pub struct CameraPage {
        pub actions: OnceCell<gio::SimpleActionGroup>,
        pub shortcut_controller: OnceCell<gtk::ShortcutController>,
        #[template_child]
        pub camera: TemplateChild<Camera>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraPage {
        const NAME: &'static str = "CameraPage";
        type Type = super::CameraPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CameraPage { }

    impl WidgetImpl for CameraPage { }
    impl BoxImpl for CameraPage { }
}

glib::wrapper! {
    pub struct CameraPage(ObjectSubclass<imp::CameraPage>) @extends gtk::Widget, gtk::Box;
}

impl CameraPage {
    pub fn new(actions: gio::SimpleActionGroup) -> Self {
        let page = glib::Object::new::<Self>(&[]).expect("Failed to create CameraPage");
        page.imp().actions.set(actions).unwrap();
        page.setup_widgets();
        page
    }

    pub async fn scan_from_camera(&self) -> Result<String> {
        let imp = self.imp();

        let (tx, rx) = oneshot::channel();

        // This is required because for whatever reason `glib::clone!` wouldn't let it be moved into
        // the closure.
        let tx = Rc::new(Cell::new(Some(tx)));

        // This is to make it safe to access `src` inside of the connected closure to
        // disconnect it after being called.
        let src = Rc::new(Cell::new(None));

        src.set(Some(imp.camera.connect_local(
            "code-detected",
            false,
            clone!(
                @weak self as camera_page, @strong src, @strong tx
                => @default-return None, move |arguments| {
                    let code = arguments[1].get::<String>().unwrap();
                    match tx.take().unwrap().send(code) {
                        Ok(()) => (),
                        Err(_) => {
                            tracing::error!(concat!(
                                "CameraPage::scan_from_camera failed to send the resulting QR ",
                                "code to the recipient because the recipient already received a ",
                                "QR code or was dropped. This should never occur.",
                            ));
                        }
                    }
                    camera_page.imp().camera.disconnect(src.take().unwrap());
                    None
                }
            )
        )));

        drop(tx);
        drop(src);

        imp.camera.from_camera();

        match rx.await {
            Ok(code) => Ok(code),
            Err(error) => {
                tracing::error!(concat!(
                    "CameraPage::scan_from_camera failed to receive the resulting QR code from ",
                    "the sender because the sender was dropped without sending a QR code. This ",
                    "should never occur."
                ));
                Err(error.into())
            }
        }
    }

    pub async fn scan_from_screenshot(&self) -> Result<String> {
        let imp = self.imp();

        let (tx, rx) = oneshot::channel();

        // This is required because for whatever reason `glib::clone!` wouldn't let it be moved into
        // the closure.
        let tx = Rc::new(Cell::new(Some(tx)));

        // This is to make it safe to access `src` inside of the connected closure to
        // disconnect it after being called.
        let src = Rc::new(Cell::new(None));

        src.set(Some(imp.camera.connect_local(
            "code-detected",
            false,
            clone!(
                @weak self as camera_page, @strong src, @strong tx
                => @default-return None, move |arguments| {
                    let code = arguments[1].get::<String>().unwrap();
                    match tx.take().unwrap().send(code) {
                        Ok(()) => (),
                        Err(_) => {
                            tracing::error!(concat!(
                                "CameraPage::scan_from_screenshot failed to send the resulting QR ",
                                "code to the recipient because the recipient already received a ",
                                "QR code or was dropped. This should never occur.",
                            ));
                        }
                    }
                    camera_page.imp().camera.disconnect(src.take().unwrap());
                    None
                }
            )
        )));

        drop(tx);
        drop(src);

        imp.camera.from_screenshot().await?;

        match rx.await {
            Ok(code) => Ok(code),
            Err(error) => {
                tracing::error!(concat!(
                    "CameraPage::scan_from_camera failed to receive the resulting QR code from ",
                    "the sender because the sender was dropped without sending a QR code. This ",
                    "should never occur."
                ));
                Err(error.into())
            }
        }
    }

    fn setup_widgets(&self) {
        let imp = self.imp();
        let actions = imp.actions.get().unwrap();

        imp.camera.connect_local(
            "close",
            false,
            clone!(@weak actions => @default-return None, move |_| {
                get_action!(actions, @close_page).activate(None);
                None
            })
        );
    }
}

