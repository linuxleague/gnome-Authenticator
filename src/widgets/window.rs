use crate::application::Action;
use crate::config::{APP_ID, PROFILE};
use crate::widgets::providers::ProvidersList;
use crate::window_state;

use gio::prelude::*;
use glib::{signal::Inhibit, Sender};
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Debug)]
pub enum View {
    Empty,
    Locked,
    Accounts,
}

pub struct Window {
    pub widget: libhandy::ApplicationWindow,
    sender: Sender<Action>,
    builder: gtk::Builder,
    settings: RefCell<gio::Settings>,
}

impl Window {
    pub fn new(sender: Sender<Action>) -> Rc<Self> {
        let settings = gio::Settings::new(APP_ID);
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/window.ui");
        get_widget!(builder, libhandy::ApplicationWindow, window);

        if PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        let window_widget = Rc::new(Window {
            widget: window,
            sender,
            builder,
            settings: RefCell::new(settings),
        });

        window_widget.init();
        window_widget.setup_actions();
        window_widget.set_view(View::Empty); // Start by default in an empty state
        window_widget
    }

    pub fn set_view(&self, view: View) {
        /*get_widget!(self.builder, gtk::Stack, headerbar_stack);
        get_widget!(self.builder, gtk::Stack, main_stack);
        match view {
            View::Empty => {
                main_stack.set_visible_child_name("empty_state");
                headerbar_stack.set_visible_child_name("empty_headerbar");
            }
            View::Locked => {
                main_stack.set_visible_child_name("locked_state");
                headerbar_stack.set_visible_child_name("locked_headerbar");
            }
            View::Accounts => {
                main_stack.set_visible_child_name("normal_state");
                headerbar_stack.set_visible_child_name("main_headerbar");
            }
        }*/
    }

    fn init(&self) {
        // load latest window state
        let settings = self.settings.borrow().clone();
        window_state::load(&self.widget, &settings);
        // save window state on delete event
        self.widget.connect_close_request(move |window| {
            window_state::save(&window, &settings);
            Inhibit(false)
        });

        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.widget.set_help_overlay(Some(&shortcuts));

        let providers_list = ProvidersList::new(self.sender.clone());
        /*get_widget!(self.builder, gtk::Box, providers_container);

        providers_container.append(&providers_list.widget);
        if providers_list.model.borrow().get_count() != 0 {
            send!(self.sender, Action::ViewAccounts);
        }*/
    }

    fn setup_actions(&self) {
        action!(
            self.widget,
            "add-account",
            clone!(@strong self.sender as sender => move |_,_| {
                send!(sender, Action::OpenAddAccountDialog);
            })
        );
    }
}
