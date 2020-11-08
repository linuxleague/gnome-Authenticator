use crate::config;
use crate::models::Account;
use crate::widgets::{AddAccountDialog, View, Window};
use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::{ApplicationImpl, ApplicationImplExt, GtkApplicationImpl};
use std::cell::RefCell;
use std::env;

use glib::{Receiver, Sender};
pub enum Action {
    ViewAccounts,
    AccountCreated(Account),
    OpenAddAccountDialog,
}

pub struct ApplicationPrivate {
    window: RefCell<Option<Window>>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
}

impl ObjectSubclass for ApplicationPrivate {
    const NAME: &'static str = "Application";
    type ParentType = gtk::Application;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        Self {
            window: RefCell::new(None),
            sender,
            receiver,
        }
    }
}

impl ObjectImpl for ApplicationPrivate {}
impl GtkApplicationImpl for ApplicationPrivate {}
impl ApplicationImpl for ApplicationPrivate {
    fn startup(&self, application: &gio::Application) {
        self.parent_startup(application);
        let app_ = ObjectSubclass::get_instance(self)
            .downcast::<Application>()
            .unwrap();
        libhandy::functions::init();

        if let Some(ref display) = gdk::Display::get_default() {
            let p = gtk::CssProvider::new();
            gtk::CssProvider::load_from_resource(&p, "/com/belmoussaoui/Authenticator/style.css");
            gtk::StyleContext::add_provider_for_display(display, &p, 500);
        }
        application.set_resource_base_path(Some("/com/belmoussaoui/Authenticator"));

        action!(
            application,
            "quit",
            clone!(@strong application as app => move |_, _| app.quit())
        );
        // About
        action!(
            application,
            "about",
            clone!(@strong app_ => move |_, _| {
                let window = app_.get_active_window().unwrap();

                let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/about_dialog.ui");
                get_widget!(builder, gtk::AboutDialog, about_dialog);
                about_dialog.set_transient_for(Some(&window));
                about_dialog.show();
            })
        );
        app_.set_accels_for_action("app.quit", &["<primary>q"]);
        app_.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
    }

    fn activate(&self, _app: &gio::Application) {
        if let Some(ref win) = *self.window.borrow() {
            win.present();
            return;
        }

        let app = ObjectSubclass::get_instance(self)
            .downcast::<Application>()
            .unwrap();
        let window = app.create_window();
        window.present();
        self.window.replace(Some(window));

        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(None, move |action| app.do_action(action));
    }
}

glib_wrapper! {
    pub struct Application(
        Object<subclass::simple::InstanceStruct<ApplicationPrivate>,
        subclass::simple::ClassStruct<ApplicationPrivate>>)
        @extends gio::Application, gtk::Application, gio::ActionMap;

    match fn {
        get_type => || ApplicationPrivate::get_type().to_glib(),
    }
}

impl Application {
    pub fn run() {
        info!("Authenticator ({})", config::APP_ID);
        info!("Version: {} ({})", config::VERSION, config::PROFILE);
        info!("Datadir: {}", config::PKGDATADIR);

        let app = glib::Object::new(
            Application::static_type(),
            &[
                ("application-id", &Some(config::APP_ID)),
                ("flags", &gio::ApplicationFlags::empty()),
            ],
        )
        .unwrap()
        .downcast::<Application>()
        .unwrap();

        let args: Vec<String> = env::args().collect();
        ApplicationExtManual::run(&app, &args);
    }

    fn create_window(&self) -> Window {
        let self_ = ApplicationPrivate::from_instance(self);
        let window = Window::new(self_.sender.clone(), &self.clone());

        window
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        let self_ = ApplicationPrivate::from_instance(self);
        match action {
            Action::OpenAddAccountDialog => {
                let dialog = AddAccountDialog::new(self_.sender.clone());
                //dialog.widget.set_transient_for(Some(&self_.window));
                dialog.widget.show();
            }
            Action::ViewAccounts => (), //self.set_view(View::Accounts),
            Action::AccountCreated(account) => {
                println!("{:#?}", account);
            }
        };

        glib::Continue(true)
    }
}
