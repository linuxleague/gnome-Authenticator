use crate::application::Action;
use crate::models::Account;
use glib::Sender;
use gtk::prelude::*;

pub struct AccountRow {
    pub widget: gtk::ListBoxRow,
    builder: gtk::Builder,
    sender: Sender<Action>,
    account: Account,
}

impl AccountRow {
    pub fn new(account: Account, sender: Sender<Action>) -> Self {
        let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/account_row.ui");
        let widget: gtk::ListBoxRow = builder.get_object("account_row").expect("Failed to load library_row object");

        let account_row = Self { widget, builder, sender, account };
        account_row.init();
        account_row
    }

    fn init(&self) {
        let username_label: gtk::Label = self.builder.get_object("username_label").expect("Failed to retrieve username_label");

        username_label.set_text(&self.account.username);
    }

    pub fn set_on_click_callback<F>(&self, callback: F)
    where
        for<'r, 's> F: std::ops::Fn(&'r gtk::EventBox, &'s gdk::EventButton) -> gtk::Inhibit + 'static,
    {
        //let event_box: gtk::EventBox = self.builder.get_object("eventbox").expect("Failed to load eventbox");
        // event_box.connect_button_press_event(callback);
    }
}
