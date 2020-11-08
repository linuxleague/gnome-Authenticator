use crate::application::Action;
use crate::models::{Account, Provider, ProvidersModel};
use crate::widgets::accounts::AccountRow;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct ProviderRow<'a> {
    pub widget: gtk::ListBoxRow,
    provider: &'a Provider,
    builder: gtk::Builder,
    sender: Sender<Action>,
}

impl<'a> ProviderRow<'a> {
    pub fn new(provider: &'a Provider, sender: Sender<Action>) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/provider_row.ui");
        get_widget!(builder, gtk::ListBoxRow, provider_row);

        let row = Self {
            widget: provider_row,
            builder,
            sender,
            provider,
        };
        row.init();
        row
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::Label, name);

        self.provider
            .bind_property("name", &name, "label")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        get_widget!(self.builder, gtk::ListBox, accounts_list);
        accounts_list.bind_model(
            Some(self.provider.accounts()),
            Some(Box::new(
                clone!(@strong self.sender as sender => move |account: &glib::Object| {
                    let account: &Account = account
                        .downcast_ref::<Account>()
                        .unwrap();
                    let row = AccountRow::new(account, sender.clone());
                    row.widget.upcast::<gtk::Widget>()
                }),
            )),
        );
    }
}
