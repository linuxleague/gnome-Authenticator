use std::{cell::Cell, rc::Rc};

use adw::subclass::prelude::*;
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
use tokio::{
    select,
    sync::oneshot,
    time::{sleep, Duration},
};

use crate::{utils::spawn_tokio, widgets::Camera};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/preferences_camera_page.ui")]
    pub struct CameraPage {
        pub actions: OnceCell<gio::SimpleActionGroup>,
        #[template_child]
        pub camera: TemplateChild<Camera>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraPage {
        const NAME: &'static str = "CameraPage";
        type Type = super::CameraPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CameraPage {}

    impl WidgetImpl for CameraPage {}
    impl BinImpl for CameraPage {}
}

glib::wrapper! {
    pub struct CameraPage(ObjectSubclass<imp::CameraPage>) @extends gtk::Widget, adw::Bin;
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

        // This is required because for whatever reason `glib::clone!` wouldn't let it
        // be moved into the closure.
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
            ),
        )));

        drop(tx);
        drop(src);

        imp.camera.scan_from_camera();

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

        // This is required because for whatever reason `glib::clone!` wouldn't let it
        // be moved into the closure.
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
            ),
        )));

        drop(tx);

        select! {
            biased;
            result = rx => result.map_err(|error| {
                tracing::error!(concat!(
                    "CameraPage::scan_from_screenshot failed to receive the resulting QR code ",
                    "from the sender because the sender was dropped without sending a QR ",
                    "code. This should never occur.",
                ));

                error.into()
            }),
            result = (|| async move {
                imp.camera.scan_from_screenshot().await?;

                // Give the GLib event loop a whole 2.5 seconds to dispatch the "code-detected"
                // action before we assume that its not going to be dispatched at all.
                spawn_tokio(async { sleep(Duration::from_millis(2500)).await; }).await;

                // Disconnect the signal handler.
                imp.camera.disconnect(src.take().unwrap());

                anyhow::bail!(concat!(
                    "CameraPage::scan_from_screenshot failed to receive the resulting QR code in ",
                    "a reasonable amount of time."
                ));
            })() => result.map_err(From::from),
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
            }),
        );
    }
}
