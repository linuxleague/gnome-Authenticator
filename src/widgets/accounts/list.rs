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
        let widget: gtk::Box = builder
            .get_object("accounts_list")
            .expect("Failed to retrieve accounts_list");
        let accounts_list = Self {
            widget,
            builder,
            sender,
            model,
            provider,
        };
        accounts_list.init();
        accounts_list
    }

    fn init(&self) {
        let provider_name: gtk::Label = self
            .builder
            .get_object("provider_name")
            .expect("Failed to retrieve provider_name");
        provider_name.set_text(&self.provider.name);

        let listbox: gtk::ListBox = self
            .builder
            .get_object("listbox")
            .expect("Failed to retrieve listbox");
        let sender = self.sender.clone();

        listbox.bind_model(
            Some(&self.model.model),
            Some(Box::new(move |account: &glib::Object| {
                let account: Account = account
                    .downcast_ref::<ObjectWrapper>()
                    .unwrap()
                    .deserialize();
                let row = AccountRow::new(account, sender.clone());
                let sender = sender.clone();
                /*row.set_on_click_callback(move |_, _| {
                    // sender.send(Action::LoadChapter(chapter.clone())).unwrap();
                    Inhibit(false)
                });*/
                row.widget.upcast::<gtk::Widget>()
            })),
        );

        listbox.set_header_func(Some(Box::new(
            move |row1: &gtk::ListBoxRow, row2: Option<&gtk::ListBoxRow>| {
                if let Some(row_before) = row2 {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    row1.set_header(Some(&separator));
                    separator.show();
                }
            },
        )));
    }
}
