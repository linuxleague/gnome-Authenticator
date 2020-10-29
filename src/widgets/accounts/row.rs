use crate::application::Action;
use crate::models::Account;
use glib::Sender;
use gtk::prelude::*;

pub struct AccountRow<'a> {
    pub widget: gtk::ListBoxRow,
    builder: gtk::Builder,
    sender: Sender<Action>,
    account: &'a Account,
}

impl<'a> AccountRow<'a> {
    pub fn new(account: &'a Account, sender: Sender<Action>) -> Self {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/account_row.ui");
        get_widget!(builder, gtk::ListBoxRow, account_row);

        let row = Self {
            widget: account_row,
            builder,
            sender,
            account,
        };
        row.init();
        row
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::Label, username_label);
        username_label.set_text(&self.account.name());
    }
}
