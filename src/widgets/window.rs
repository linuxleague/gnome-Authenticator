use crate::application::Action;
use crate::config::{APP_ID, PROFILE};
use crate::widgets::providers::ProvidersList;
use crate::window_state;

use gio::prelude::*;
use glib::Sender;
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
    pub widget: gtk::ApplicationWindow,
    sender: Sender<Action>,
    builder: gtk::Builder,
    settings: RefCell<gio::Settings>,
}

impl Window {
    pub fn new(sender: Sender<Action>) -> Rc<RefCell<Self>> {
        let settings = gio::Settings::new(APP_ID);
        let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/window.ui");
        let widget: gtk::ApplicationWindow = builder.get_object("window").unwrap();

        if PROFILE == "Devel" {
            widget.get_style_context().add_class("devel");
        }
        let window = Rc::new(RefCell::new(Window {
            widget,
            sender,
            builder,
            settings: RefCell::new(settings),
        }));

        window.borrow().init();
        window.borrow().setup_actions(window.clone());
        window.borrow().set_view(View::Empty); // Start by default in an empty state
        window
    }

    pub fn set_view(&self, view: View) {
        let headerbar_stack: gtk::Stack = self.builder.get_object("headerbar_stack").expect("Failed to retrieve headerbar_stack");
        let main_stack: gtk::Stack = self.builder.get_object("main_stack").expect("Failed to retrieve main_stack");
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
        }
    }

    fn init(&self) {
        // load latest window state
        let settings = self.settings.borrow().clone();
        window_state::load(&self.widget, &settings);
        // save window state on delete event
        self.widget.connect_delete_event(move |window, _| {
            window_state::save(&window, &settings);
            Inhibit(false)
        });

        let providers_list = ProvidersList::new(self.sender.clone());
        let providers_container: gtk::Box = self.builder.get_object("providers_container").expect("Failed to retrieve providers_container");

        providers_container.pack_start(&providers_list.widget, true, true, 0);
        if providers_list.model.borrow().get_count() != 0 {
            self.sender.send(Action::ViewAccounts).expect("Failed to send ViewAccountsAction");
        }
    }
    /*
    fn order(&self, sort_by: Option<SortBy>, sort_order: Option<SortOrder>) {
        // self.library.borrow_mut().clone().sort(sort_by.clone(), sort_order.clone());

    }
    */

    fn setup_actions(&self, s: Rc<RefCell<Self>>) {
        let actions = gio::SimpleActionGroup::new();
        let add_account = gio::SimpleAction::new("add-account", None);
        let sender = self.sender.clone();
        add_account.connect_activate(move |_, _| {
            sender.send(Action::OpenAddAccountDialog).unwrap();
        });
        actions.add_action(&add_account);
        self.widget.insert_action_group("win", Some(&actions));

        let actions = gio::SimpleActionGroup::new();
        let sort_descending = gio::SimpleAction::new("sort-descending", None);
        let sort_ascending = gio::SimpleAction::new("sort-ascending", None);

        let window = s.clone();
        let sort_ascending_action = sort_ascending.clone();
        sort_descending.connect_activate(move |action, _| {
            action.set_enabled(false);
            sort_ascending_action.set_enabled(true);
            // window.borrow_mut().order(None, Some(SortOrder::Desc));
        });
        actions.add_action(&sort_descending);

        let window = s.clone();
        let sort_descending_action = sort_descending.clone();
        sort_ascending.connect_activate(move |action, _| {
            action.set_enabled(false);
            sort_descending_action.set_enabled(true);
            // window.borrow().order(None, Some(SortOrder::Asc));
        });
        actions.add_action(&sort_ascending);

        let window = s.clone();
        let sort_by = gio::SimpleAction::new("sort-by", Some(glib::VariantTy::new("s").unwrap()));
        sort_by.connect_activate(move |_, data| {
            // let sort_by = SortBy::from(data.unwrap().get_str().unwrap());
            // window.borrow().order(Some(sort_by), None);
        });
        actions.add_action(&sort_by);

        self.widget.insert_action_group("library", Some(&actions));
    }
}
