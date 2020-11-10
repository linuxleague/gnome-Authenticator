use crate::config;
use crate::helpers::Keyring;
use crate::models::{Account, Provider, ProvidersModel};
use crate::widgets::{AddAccountDialog, PreferencesWindow, View, Window, WindowPrivate};
use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::{ApplicationImpl, ApplicationImplExt, GtkApplicationImpl};
use std::env;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use glib::{Receiver, Sender};
pub enum Action {
    AccountCreated(Account, Provider),
    AccountRemoved(Account),
    OpenAddAccountDialog,
    SetView(View),
}

pub struct ApplicationPrivate {
    window: RefCell<Option<Window>>,
    model: Rc<ProvidersModel>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
    locked: Cell<bool>,
}
static PROPERTIES: [subclass::Property; 1] = [subclass::Property("locked", |name| {
    glib::ParamSpec::boolean(name, "locked", "locked", false, glib::ParamFlags::READWRITE)
})];
impl ObjectSubclass for ApplicationPrivate {
    const NAME: &'static str = "Application";
    type ParentType = gtk::Application;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));
        let model = Rc::new(ProvidersModel::new());

        Self {
            window: RefCell::new(None),
            sender,
            receiver,
            model,
            locked: Cell::new(false),
        }
    }
}

impl ObjectImpl for ApplicationPrivate {
    fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("locked", ..) => {
                let locked = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.locked.set(locked);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("locked", ..) => Ok(self.locked.get().to_value()),
            _ => unimplemented!(),
        }
    }
}

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
            let theme = gtk::IconTheme::get_for_display(display).unwrap();
            theme.add_resource_path("/com/belmoussaoui/Authenticator/icons/");
        }

        action!(
            application,
            "quit",
            clone!(@strong application as app => move |_, _| app.quit())
        );

        action!(
            application,
            "preferences",
            clone!(@strong app_ => move |_,_| {
                let window = app_.get_active_window().unwrap();
                let preferences = PreferencesWindow::new();
                preferences.widget.set_transient_for(Some(&window));
                preferences.widget.show();
            })
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

        action!(
            application,
            "lock",
            clone!(@strong app_ => move |_, _| {
                app_.set_locked(true);
            })
        );
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

        app.set_resource_base_path(Some("/com/belmoussaoui/Authenticator"));
        app.set_accels_for_action("app.quit", &["<primary>q"]);
        app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
        app.set_accels_for_action("win.search", &["<primary>f"]);
        app.set_accels_for_action("win.add-account", &["<primary>n"]);
        app.set_property(
            "locked",
            &Keyring::has_set_password().unwrap_or_else(|_| false),
        );
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

    pub fn locked(&self) -> bool {
        let self_ = ApplicationPrivate::from_instance(self);
        return self_.locked.get();
    }

    pub fn set_locked(&self, state: bool) {
        self.set_property("locked", &state);
    }

    fn create_window(&self) -> Window {
        let self_ = ApplicationPrivate::from_instance(self);
        let window = Window::new(self_.model.clone(), self_.sender.clone(), &self.clone());

        window
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        let self_ = ApplicationPrivate::from_instance(self);
        let active_window = self.get_active_window().unwrap();

        match action {
            Action::OpenAddAccountDialog => {
                let dialog = AddAccountDialog::new(self_.model.clone(), self_.sender.clone());
                dialog.widget.set_transient_for(Some(&active_window));
                dialog.widget.show();
            }
            Action::AccountCreated(account, provider) => {
                self_.model.add_account(&account, &provider);
            }
            Action::AccountRemoved(account) => {
                let win_ = active_window.downcast_ref::<Window>().unwrap();
                let priv_ = WindowPrivate::from_instance(win_);

                self_.model.remove_account(&account);
                priv_.providers.refilter();
            }
            Action::SetView(view) => {
                let win_ = active_window.downcast_ref::<Window>().unwrap();
                win_.set_view(view);
            }
        };

        glib::Continue(true)
    }
}
