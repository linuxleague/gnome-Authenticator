use gtk::prelude::*;

use glib::{signal::Inhibit, Sender};

use crate::application::Action;
use crate::models::{Account, AccountsModel, ObjectWrapper, Provider};
use crate::widgets::accounts::AccountRow;

pub struct AccountsList<'a> {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    sender: Sender<Action>,
    model: &'a AccountsModel,
    provider: &'a Provider,
}

/*
    ProvidersList -> Vec<AccountsList>
*/
impl<'a> AccountsList<'a> {
    pub fn new(model: &'a AccountsModel, provider: &'a Provider, sender: Sender<Action>) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/accounts_list.ui");
        get_widget!(builder, gtk::Box, accounts_list);

        let accounts = Self {
            widget: accounts_list,
            builder,
            sender,
            model,
            provider,
        };
        accounts.init();
        accounts
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::Label, provider_name);
        provider_name.set_text(&self.provider.name);

        get_widget!(self.builder, gtk::ListBox, listbox);
        listbox.bind_model(
            Some(&self.model.model),
            Some(Box::new(
                clone!(@strong self.sender as sender => move |account: &glib::Object| {
                    let account: Account = account
                        .downcast_ref::<ObjectWrapper>()
                        .unwrap()
                        .deserialize();
                    let row = AccountRow::new(account, sender.clone());
                    /*row.set_on_click_callback(move |_, _| {
                        // sender.send(Action::LoadChapter(chapter.clone())).unwrap();
                        Inhibit(false)
                    });*/
                    row.widget.upcast::<gtk::Widget>()
                }),
            )),
        );

        listbox.set_header_func(Some(Box::new(
            move |row1: &gtk::ListBoxRow, row2: Option<&gtk::ListBoxRow>| {
                if let Some(row_before) = row2 {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    row1.set_header(Some(&separator));
                }
            },
        )));
    }
}
