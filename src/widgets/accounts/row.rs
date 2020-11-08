use crate::application::Action;
use crate::models::Account;
use gio::ActionMapExt;
use glib::Sender;
use gtk::prelude::*;

pub struct AccountRow<'a> {
    pub widget: gtk::ListBoxRow,
    builder: gtk::Builder,
    sender: Sender<Action>,
    account: &'a Account,
    actions: gio::SimpleActionGroup,
}

impl<'a> AccountRow<'a> {
    pub fn new(account: &'a Account, sender: Sender<Action>) -> Self {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/account_row.ui");
        get_widget!(builder, gtk::ListBoxRow, account_row);
        let actions = gio::SimpleActionGroup::new();
        let row = Self {
            widget: account_row,
            builder,
            sender,
            account,
            actions,
        };
        row.init();
        row
    }

    fn init(&self) {
        self.widget
            .insert_action_group("account", Some(&self.actions));

        get_widget!(self.builder, gtk::Label, username_label);

        self.account
            .bind_property("name", &username_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        action!(
            self.actions,
            "delete",
            clone!(@strong self.sender as sender, @strong self.account as account => move |_, _| {
                send!(sender, Action::AccountRemoved(account.clone()));
            })
        );
    }
}
