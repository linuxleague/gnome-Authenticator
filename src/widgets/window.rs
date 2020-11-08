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

#[derive(PartialEq, Debug)]
pub enum View {
    Locked,
    Accounts,
}

pub struct WindowPrivate {
    builder: gtk::Builder,
    settings: gio::Settings,
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

        Self { builder, settings }
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
    pub fn new(sender: Sender<Action>, app: &Application) -> Self {
        let window = glib::Object::new(Window::static_type(), &[("application", app)])
            .unwrap()
            .downcast::<Window>()
            .unwrap();
        app.add_window(&window);

        if PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(sender.clone());
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

    fn init(&self, sender: Sender<Action>) {
        let self_ = WindowPrivate::from_instance(self);
        // load latest window state
        window_state::load(&self, &self_.settings);
        // save window state on delete event
        self.connect_close_request(clone!(@strong self_.settings as settings => move |window| {
            window_state::save(&window, &settings);
            Inhibit(false)
        }));

        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.set_help_overlay(Some(&shortcuts));

        let providers_model = ProvidersModel::new();

        get_widget!(self_.builder, libhandy::Leaflet, deck);
        let providers_list = ProvidersList::new(&providers_model, sender.clone());
        get_widget!(self_.builder, gtk::Box, container);
        container.append(&providers_list.widget);
        /*get_widget!(self.builder, gtk::Box, providers_container);

        providers_container.append(&providers_list.widget);
        if providers_list.model.borrow().get_count() != 0 {
            send!(self.sender, Action::ViewAccounts);
        }*/

        libhandy::ApplicationWindowExt::set_child(self, Some(&deck));
    }

    fn setup_actions(&self, sender: Sender<Action>) {
        action!(
            self,
            "add-account",
            clone!(@strong sender => move |_,_| {
                send!(sender, Action::OpenAddAccountDialog);
            })
        );
    }
}
