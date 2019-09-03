use crate::application::Action;
use crate::models::database::{self, *};
use crate::models::{Account, AccountsModel, NewAccount};
use gio::prelude::*;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct AddAccountDialog {
    pub widget: gtk::Window,
    builder: gtk::Builder,
    sender: Sender<Action>,
}

impl AddAccountDialog {
    pub fn new(sender: Sender<Action>) -> Rc<Self> {
        let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/add_account.ui");
        let widget: gtk::Window = builder.get_object("add_dialog").expect("Failed to retrieve AddAccountDialog");
        widget.show_all();

        let add_account_dialog = Rc::new(Self { widget, builder, sender });

        add_account_dialog.setup_actions(add_account_dialog.clone());
        add_account_dialog.setup_signals();
        add_account_dialog
    }

    fn add_account(&self, account: NewAccount) -> Result<Account, database::Error> {
        // TODO: add the account to the provider model.
        account.insert()
    }

    fn notify_err(&self, error_msg: &str) {
        let notification: gtk::Revealer = self.builder.get_object("notification").expect("Failed to retrieve notification");

        let notification_msg: gtk::Label = self.builder.get_object("notification_msg").expect("Failed to retrieve notification_msg");

        notification_msg.set_text(error_msg);
        notification.set_reveal_child(true); // Display the notification
    }

    fn setup_signals(&self) {
        let username_entry: gtk::Entry = self.builder.get_object("username_entry").expect("Failed to retrieve username_entry");
        let token_entry: gtk::Entry = self.builder.get_object("token_entry").expect("Failed to retrieve token_entry");

        let action_group = self.widget.get_action_group("add").unwrap().downcast::<gio::SimpleActionGroup>().unwrap();
        let save_action = action_group.lookup_action("save").unwrap().downcast::<gio::SimpleAction>().unwrap();

        let weak_username = username_entry.downgrade();
        let weak_token = token_entry.downgrade();
        let validate_entries = move |entry: &gtk::Entry| {
            let mut username = String::new();
            let mut token = String::new();

            if let Some(username_entry) = weak_username.upgrade() {
                username.push_str(&username_entry.get_text().unwrap());
            }
            if let Some(token_entry) = weak_token.upgrade() {
                token.push_str(&token_entry.get_text().unwrap());
            }

            let is_valid = !(username.is_empty() || token.is_empty());
            save_action.set_enabled(is_valid);
        };

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
        let add_account_dialog = s.clone();
        save.connect_activate(move |_, _| {
            let builder = &add_account_dialog.builder;
            let username_entry: gtk::Entry = builder.get_object("username_entry").expect("Failed to retrieve username_entry");
            let token_entry: gtk::Entry = builder.get_object("token_entry").expect("Failed to retrieve token_entry");

            let new_account = NewAccount {
                username: username_entry.get_text().unwrap().to_string(),
                token_id: token_entry.get_text().unwrap().to_string(),
                provider: 1,
            };
            if let Err(err) = add_account_dialog.add_account(new_account) {
                add_account_dialog.notify_err("Failed to add a new account");
            } else {
                // Close the dialog if everything is fine.
                add_account_dialog.widget.destroy();
            }
        });
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
}
