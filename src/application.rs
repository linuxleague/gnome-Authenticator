use crate::config;
use crate::helpers::Keyring;
use crate::models::{Account, Provider, ProvidersModel};
use crate::widgets::{AccountAddDialog, PreferencesWindow, ProvidersDialog, View, Window};
use gio::prelude::*;
use glib::subclass::prelude::*;
use glib::{subclass, WeakRef};
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
    window: RefCell<Option<WeakRef<Window>>>,
    model: Rc<ProvidersModel>,
    sender: Sender<Action>,
    receiver: RefCell<Option<Receiver<Action>>>,
    locked: Cell<bool>,
    can_be_locked: Cell<bool>,
}
static PROPERTIES: [subclass::Property; 2] = [
    subclass::Property("locked", |name| {
        glib::ParamSpec::boolean(name, "locked", "locked", false, glib::ParamFlags::READWRITE)
    }),
    subclass::Property("can-be-locked", |name| {
        glib::ParamSpec::boolean(
            name,
            "can_be_locked",
            "can be locked",
            false,
            glib::ParamFlags::READWRITE,
        )
    }),
];
impl ObjectSubclass for ApplicationPrivate {
    const NAME: &'static str = "Application";
    type ParentType = gtk::Application;
    type Type = super::Application;
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
            can_be_locked: Cell::new(false),
            locked: Cell::new(false),
        }
    }
}

impl ObjectImpl for ApplicationPrivate {
    fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("locked", ..) => {
                let locked = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.locked.set(locked);
            }
            subclass::Property("can-be-locked", ..) => {
                let can_be_locked = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.can_be_locked.set(can_be_locked);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("locked", ..) => self.locked.get().to_value(),
            subclass::Property("can-be-locked", ..) => self.can_be_locked.get().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl GtkApplicationImpl for ApplicationPrivate {}
impl ApplicationImpl for ApplicationPrivate {
    fn startup(&self, app: &Self::Type) {
        self.parent_startup(app);

        libhandy::functions::init();

        let app = app.downcast_ref::<Application>().unwrap();
        if let Some(ref display) = gdk::Display::get_default() {
            let p = gtk::CssProvider::new();
            gtk::CssProvider::load_from_resource(&p, "/com/belmoussaoui/Authenticator/style.css");
            gtk::StyleContext::add_provider_for_display(display, &p, 500);
            let theme = gtk::IconTheme::get_for_display(display).unwrap();
            theme.add_resource_path("/com/belmoussaoui/Authenticator/icons/");
        }

        action!(app, "quit", clone!(@strong app => move |_, _| app.quit()));

        action!(
            app,
            "preferences",
            clone!(@strong app => move |_,_| {
                let window = app.get_active_window().unwrap();
                let preferences = PreferencesWindow::new();
                preferences.set_transient_for(Some(&window));
                preferences.show();
            })
        );

        // About
        action!(
            app,
            "about",
            clone!(@strong app => move |_, _| {
                let window = app.get_active_window().unwrap();

                let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/about_dialog.ui");
                get_widget!(builder, gtk::AboutDialog, about_dialog);
                about_dialog.set_transient_for(Some(&window));
                about_dialog.show();
            })
        );

        action!(
            app,
            "providers",
            clone!(@strong app, @weak self.model as model => move |_, _| {
                let window = app.get_active_window().unwrap();
                let providers = ProvidersDialog::new(model);
                providers.set_transient_for(Some(&window));
                providers.show();
            })
        );

        action!(
            app,
            "lock",
            clone!(@strong app => move |_, _| {
                app.set_locked(true);
            })
        );
        app.bind_property("can-be-locked", &get_action!(app, @lock), "enabled")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();
        app.bind_property("locked", &get_action!(app, @preferences), "enabled")
            .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
            .build();
        app.bind_property("locked", &get_action!(app, @providers), "enabled")
            .flags(glib::BindingFlags::INVERT_BOOLEAN | glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    fn activate(&self, app: &Self::Type) {
        if let Some(ref win) = *self.window.borrow() {
            let window = win.upgrade().unwrap();
            window.present();
            return;
        }

        let app = app.downcast_ref::<Application>().unwrap();
        let window = app.create_window();
        window.present();
        self.window.replace(Some(window.downgrade()));
        let has_set_password = Keyring::has_set_password().unwrap_or_else(|_| false);

        app.set_resource_base_path(Some("/com/belmoussaoui/Authenticator"));
        app.set_accels_for_action("app.quit", &["<primary>q"]);
        app.set_accels_for_action("app.lock", &["<primary>l"]);
        app.set_accels_for_action("app.providers", &["<primary>p"]);
        app.set_accels_for_action("app.preferences", &["<primary>comma"]);
        app.set_accels_for_action("win.show-help-overlay", &["<primary>question"]);
        app.set_accels_for_action("win.search", &["<primary>f"]);
        app.set_accels_for_action("win.add-account", &["<primary>n"]);

        app.set_locked(has_set_password);
        app.set_can_be_locked(has_set_password);
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong app => move |action| app.do_action(action)),
        );
    }
}

glib_wrapper! {
    pub struct Application(ObjectSubclass<ApplicationPrivate>)
        @extends gio::Application, gtk::Application, gio::ActionMap;
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
        self.set_property("locked", &state)
            .expect("Failed to set locked property");
    }

    pub fn set_can_be_locked(&self, state: bool) {
        self.set_property("can-be-locked", &state)
            .expect("Failed to set can-be-locked property");
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
                let dialog = AccountAddDialog::new(self_.model.clone(), self_.sender.clone());
                dialog.set_transient_for(Some(&active_window));
                dialog.show();
            }
            Action::AccountCreated(account, provider) => {
                let win = active_window.downcast_ref::<Window>().unwrap();

                self_.model.add_account(&account, &provider);
                win.providers().refilter();
            }
            Action::AccountRemoved(account) => {
                let win = active_window.downcast_ref::<Window>().unwrap();

                self_.model.remove_account(&account);
                win.providers().refilter();
            }
            Action::SetView(view) => {
                let win_ = active_window.downcast_ref::<Window>().unwrap();
                win_.set_view(view);
            }
        };

        glib::Continue(true)
    }
}
