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

        get_widget!(self.builder, gtk::Label, name_label);
        get_widget!(self.builder, gtk::Entry, name_entry);

        self.account
            .bind_property("name", &name_label, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.account
            .bind_property("name", &name_entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        action!(
            self.actions,
            "delete",
            clone!(@strong self.sender as sender, @strong self.account as account => move |_, _| {
                send!(sender, Action::AccountRemoved(account.clone()));
            })
        );

        action!(
            self.actions,
            "edit",
            clone!(@strong self.builder as builder => move |_, _| {
                get_widget!(builder, gtk::Stack, edit_stack);
                edit_stack.set_visible_child_name("edit");
            })
        );

        action!(
            self.actions,
            "save",
            clone!(@weak name_entry,
                @strong self.account as account,
                @strong self.builder as builder => move |_, _| {
                let new_name = name_entry.get_text().unwrap();
                account.set_name(&new_name);

                get_widget!(builder, gtk::Stack, edit_stack);
                edit_stack.set_visible_child_name("display");
            })
        );

        name_entry.connect_changed(clone!(@strong self.actions as actions => move |entry| {
            let name = entry.get_text().unwrap();
            get_action!(actions, @save).set_enabled(!name.is_empty());
        }));
    }
}
