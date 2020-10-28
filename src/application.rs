use crate::config;
use crate::widgets::{AddAccountDialog, View, Window};
use gio::prelude::*;
use gtk::prelude::*;

use std::env;
use std::{cell::RefCell, rc::Rc};

use glib::{Receiver, Sender};
pub enum Action {
    ViewAccounts,
    OpenAddAccountDialog,
}

pub struct Application {
    app: gtk::Application,
    window: Rc<Window>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
}

impl Application {
    pub fn new() -> Rc<Self> {
        let app = gtk::Application::new(Some(config::APP_ID), Default::default()).unwrap();

        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        let window = Window::new(sender.clone());

        let application = Rc::new(Self {
            app,
            window,
            sender,
            receiver,
        });

        application.setup_gactions();
        application.setup_signals();
        application.setup_css();
        application
    }

    pub fn run(&self, app: Rc<Self>) {
        info!("Authenticator ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        let app = app.clone();
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(None, move |action| app.do_action(action));

        let args: Vec<String> = env::args().collect();
        self.app.run(&args);
    }

    fn setup_gactions(&self) {
        // Quit
        action!(
            self.app,
            "quit",
            clone!(@strong self.app as app => move |_, _| app.quit())
        );
        // About
        action!(
            self.app,
            "about",
            clone!(@weak self.window.widget as window => move |_, _| {
                let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/about_dialog.ui");
                get_widget!(builder, gtk::AboutDialog, about_dialog);
                about_dialog.set_transient_for(Some(&window));
                about_dialog.show();
            })
        );

        self.app.set_accels_for_action("app.quit", &["<primary>q"]);
        self.app
            .set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
    }

    fn setup_signals(&self) {
        self.app
            .connect_activate(clone!(@weak self.window.widget as window => move |app| {
                window.set_application(Some(app));
                app.add_window(&window);
                window.present();
            }));
    }

    fn setup_css(&self) {
        self.app
            .set_resource_base_path(Some("/com/belmoussaoui/Authenticator"));

        if let Some(ref display) = gdk::Display::get_default() {
            let p = gtk::CssProvider::new();
            gtk::CssProvider::load_from_resource(&p, "/com/belmoussaoui/Authenticator/style.css");
            gtk::StyleContext::add_provider_for_display(display, &p, 500);
        }
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        match action {
            Action::OpenAddAccountDialog => {
                let dialog = AddAccountDialog::new(self.sender.clone());
                dialog.widget.set_transient_for(Some(&self.window.widget));
                dialog.widget.show();
            }
            Action::ViewAccounts => self.window.set_view(View::Accounts),
        };

        glib::Continue(true)
    }
}
