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
    window: Rc<RefCell<Window>>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
}

impl Application {
    pub fn new() -> Rc<Self> {
        let app = gtk::Application::new(Some(config::APP_ID), Default::default()).unwrap();

        glib::set_application_name(&format!("{}Authenticator", config::NAME_PREFIX));
        glib::set_prgname(Some("authenticator"));

        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        let window = Window::new(sender.clone());

        let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        let dialog: gtk::ShortcutsWindow = builder.get_object("shortcuts").unwrap();
        window.borrow().widget.set_help_overlay(Some(&dialog));

        let application = Rc::new(Self { app, window, sender, receiver });

        application.setup_gactions();
        application.setup_signals();
        application.setup_css();
        application
    }

    pub fn run(&self, app: Rc<Self>) {
        info!("{}Authenticator ({})", config::NAME_PREFIX, config::APP_ID);
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
        let app = self.app.clone();
        let simple_action = gio::SimpleAction::new("quit", None);
        simple_action.connect_activate(move |_, _| app.quit());
        self.app.add_action(&simple_action);
        self.app.set_accels_for_action("app.quit", &["<primary>q"]);

        // About
        let window = self.window.borrow().widget.clone();
        let simple_action = gio::SimpleAction::new("about", None);
        simple_action.connect_activate(move |_, _| {
            let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/about_dialog.ui");
            let about_dialog: gtk::AboutDialog = builder.get_object("about_dialog").unwrap();
            about_dialog.set_transient_for(Some(&window));

            about_dialog.connect_response(|dialog, _| dialog.destroy());
            about_dialog.show();
        });
        self.app.add_action(&simple_action);

        self.app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
    }

    fn setup_signals(&self) {
        let window = self.window.borrow().widget.clone();
        self.app.connect_activate(move |app| {
            let gtk_settings = gtk::Settings::get_default().unwrap();

            window.set_application(Some(app));
            app.add_window(&window);
            window.present();
        });
    }

    fn setup_css(&self) {
        let theme = gtk::IconTheme::get_default().unwrap();
        theme.add_resource_path("/com/belmoussaoui/Authenticator/icons");

        let p = gtk::CssProvider::new();
        gtk::CssProvider::load_from_resource(&p, "/com/belmoussaoui/Authenticator/style.css");
        gtk::StyleContext::add_provider_for_screen(&gdk::Screen::get_default().unwrap(), &p, 500);
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        match action {
            Action::OpenAddAccountDialog => {
                let dialog = AddAccountDialog::new(self.sender.clone());
                dialog.widget.set_transient_for(Some(&self.window.borrow().widget));
            }
            Action::ViewAccounts => self.window.borrow().set_view(View::Accounts),
        };

        glib::Continue(true)
    }
}
