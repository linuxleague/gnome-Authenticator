use crate::application::{Action, Application};
use crate::config::{APP_ID, PROFILE};
use crate::models::ProvidersModel;
use crate::widgets::providers::ProvidersList;
use crate::window_state;
use gio::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use glib::{signal::Inhibit, Sender};
use gtk::prelude::*;
use gtk::subclass::prelude::{WidgetImpl, WindowImpl};
use libhandy::prelude::*;
use std::rc::Rc;

#[derive(PartialEq, Debug)]
pub enum View {
    Locked,
    Accounts,
}

pub struct WindowPrivate {
    builder: gtk::Builder,
    settings: gio::Settings,
    pub providers: Rc<ProvidersList>,
}

impl ObjectSubclass for WindowPrivate {
    const NAME: &'static str = "Window";
    type ParentType = libhandy::ApplicationWindow;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/window.ui");
        let settings = gio::Settings::new(APP_ID);
        let providers = Rc::new(ProvidersList::new());
        Self {
            builder,
            settings,
            providers,
        }
    }
}

impl ObjectImpl for WindowPrivate {}

impl WidgetImpl for WindowPrivate {}

impl WindowImpl for WindowPrivate {}

impl gtk::subclass::prelude::ApplicationWindowImpl for WindowPrivate {}

impl libhandy::subclass::prelude::ApplicationWindowImpl for WindowPrivate {}

glib_wrapper! {
    pub struct Window(
        Object<subclass::simple::InstanceStruct<WindowPrivate>,
        subclass::simple::ClassStruct<WindowPrivate>>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, libhandy::ApplicationWindow, gio::ActionMap;

    match fn {
        get_type => || WindowPrivate::get_type().to_glib(),
    }
}

impl Window {
    pub fn new(model: Rc<ProvidersModel>, sender: Sender<Action>, app: &Application) -> Self {
        let window = glib::Object::new(Window::static_type(), &[("application", app)])
            .unwrap()
            .downcast::<Window>()
            .unwrap();
        app.add_window(&window);

        if PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(model, sender.clone());
        window.setup_actions(sender.clone());
        window.set_view(View::Accounts); // Start by default in an empty state
        window
    }

    pub fn set_view(&self, view: View) {
        let self_ = WindowPrivate::from_instance(self);
        get_widget!(self_.builder, libhandy::Leaflet, deck);
        match view {
            View::Locked => {
                //main_stack.set_visible_child_name("locked_state");
                //headerbar_stack.set_visible_child_name("locked_headerbar");
            }
            View::Accounts => {
                deck.set_visible_child_name("accounts");
            }
        }
    }

    fn init(&self, model: Rc<ProvidersModel>, sender: Sender<Action>) {
        let self_ = WindowPrivate::from_instance(self);
        self_.providers.set_model(model.clone());
        self_.providers.init(sender.clone());
        // load latest window state
        window_state::load(&self, &self_.settings);
        // save window state on delete event
        self.connect_close_request(clone!(@strong self_.settings as settings => move |window| {
            if let Err(err) = window_state::save(&window, &settings) {
                warn!("Failed to save window state {:#?}", err);
            }
            Inhibit(false)
        }));

        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.set_help_overlay(Some(&shortcuts));

        get_widget!(self_.builder, gtk::Box, container);
        container.append(&self_.providers.widget);

        get_widget!(self_.builder, gtk::SearchBar, search_bar);
        get_widget!(self_.builder, gtk::ToggleButton, search_btn);
        search_btn
            .bind_property("active", &search_bar, "search-mode-enabled")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();
        get_widget!(self_.builder, gtk::SearchEntry, search_entry);

        search_entry.connect_search_changed(
            clone!(@weak self_.providers as providers => move |entry| {
                let text = entry.get_text().unwrap().to_string();
                providers.search(text);
            }),
        );

        search_entry.connect_stop_search(clone!(@strong search_btn => move |entry| {
            entry.set_text("");
            search_btn.set_active(false);
        }));

        get_widget!(self_.builder, libhandy::Leaflet, deck);
        libhandy::ApplicationWindowExt::set_child(self, Some(&deck));

        let gtk_settings = gtk::Settings::get_default().unwrap();
        self_.settings.bind(
            "dark-theme",
            &gtk_settings,
            "gtk-application-prefer-dark-theme",
            gio::SettingsBindFlags::DEFAULT,
        );
    }

    fn setup_actions(&self, sender: Sender<Action>) {
        let self_ = WindowPrivate::from_instance(self);
        action!(
            self,
            "search",
            clone!(@strong self_.builder as builder => move |_,_| {
                get_widget!(builder, gtk::ToggleButton, search_btn);
                search_btn.set_active(!search_btn.get_active());
            })
        );

        action!(
            self,
            "add-account",
            clone!(@strong sender => move |_,_| {
                send!(sender, Action::OpenAddAccountDialog);
            })
        );
    }
}
