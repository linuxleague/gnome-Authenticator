use crate::application::Action;
use crate::models::database::{self, *};
use crate::models::{Account, NewAccount};
use gio::prelude::*;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct AddAccountDialog {
    pub widget: libhandy::Window,
    builder: gtk::Builder,
    sender: Sender<Action>,
}

impl AddAccountDialog {
    pub fn new(sender: Sender<Action>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/add_account.ui");
        let widget: libhandy::Window = builder
            .get_object("add_dialog")
            .expect("Failed to retrieve AddAccountDialog");

        let add_account_dialog = Rc::new(Self {
            widget,
            builder,
            sender,
        });

        add_account_dialog.setup_actions(add_account_dialog.clone());
        add_account_dialog.setup_signals();
        add_account_dialog.setup_widgets();
        add_account_dialog
    }

    fn add_account(&self, account: NewAccount) -> Result<Account, database::Error> {
        // TODO: add the account to the provider model.
        account.insert()
    }

    fn notify_err(&self, error_msg: &str) {
        get_widget!(self.builder, gtk::Revealer, notification);
        get_widget!(self.builder, gtk::Label, notification_msg);

        notification_msg.set_text(error_msg);
        notification.set_reveal_child(true); // Display the notification
    }

    fn setup_signals(&self) {
        get_widget!(self.builder, gtk::Entry, username_entry);
        get_widget!(self.builder, gtk::Entry, token_entry);

        //let action_group = self.widget.get_action_group("add").unwrap().downcast::<gio::SimpleActionGroup>().unwrap();
        //let save_action = action_group.lookup_action("save").unwrap().downcast::<gio::SimpleAction>().unwrap();

        let validate_entries = clone!(@weak username_entry, @weak token_entry => move |_: &gtk::Entry| {
            let username = username_entry.get_text().unwrap();
            let token = token_entry.get_text().unwrap();

            let is_valid = !(username.is_empty() || token.is_empty());
            //save_action.set_enabled(is_valid);
        });

        username_entry.connect_changed(validate_entries.clone());
        token_entry.connect_changed(validate_entries);
    }

    fn setup_actions(&self, s: Rc<Self>) {
        let actions = gio::SimpleActionGroup::new();
        let back = gio::SimpleAction::new("back", None);
        let sender = self.sender.clone();

        let weak_dialog = self.widget.downgrade();
        back.connect_activate(move |_, _| {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.destroy();
            }
        });
        actions.add_action(&back);

        let save = gio::SimpleAction::new("save", None);

        save.connect_activate(clone!(@strong self.builder as builder => move |_, _| {
            get_widget!(builder, gtk::Entry, username_entry);
            get_widget!(builder, gtk::Entry, token_entry);
            get_widget!(builder, gtk::Entry, provider_entry);
            get_widget!(builder, gtk::Entry, website_entry);
            // get_widget!(builder, gtk::Entry, period_entry);
            // get_widget!(builder, gtk::Entry, algorithm_model);







            /*
            let new_account = NewAccount {
                username: username_entry.get_text().unwrap().to_string(),
                token_id: token_entry.get_text().unwrap().to_string(),
                provider: provider_combobox.get_active_id().unwrap().parse::<i32>().unwrap(),
            };
            if let Err(err) = add_account_dialog.add_account(new_account) {
                add_account_dialog.notify_err("Failed to add a new account");
            } else {
                // Close the dialog if everything is fine.
                add_account_dialog.widget.destroy();
            }
            */
        }));
        save.set_enabled(false);
        actions.add_action(&save);

        let scan_qr = gio::SimpleAction::new("scan-qr", None);
        let sender = self.sender.clone();
        scan_qr.connect_activate(move |_, _| {
            // sender.send(Action::OpenAddAccountDialog).unwrap();
        });
        actions.add_action(&scan_qr);
        self.widget.insert_action_group("add", Some(&actions));
    }

    fn setup_widgets(&self) {
        // Fill the providers gtk::ListStore
        /*get_widget!(self.builder, gtk::ListStore, providers_store);
        if let Ok(providers) = database::get_providers() {
            for provider in providers.iter() {
                let values: [&dyn ToValue; 2] = [&provider.id, &provider.name];
                providers_store.set(&providers_store.append(), &[0, 1], &values);
            }
        }*/

        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton).set_value(30.0);
    }
}
