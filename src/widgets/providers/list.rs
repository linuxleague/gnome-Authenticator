use gio::prelude::*;
use glib::Sender;
use gtk::prelude::*;
use std::cell::RefCell;

use crate::application::Action;
use crate::models::ProvidersModel;
use crate::widgets::AccountsList;

pub struct ProvidersList {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    sender: Sender<Action>,
    pub model: RefCell<ProvidersModel>,
}

impl ProvidersList {
    pub fn new(sender: Sender<Action>) -> Self {
        let builder = gtk::Builder::new_from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
        let widget: gtk::Box = builder.get_object("providers_list").expect("Failed to retrieve providers_list");

        let model = RefCell::new(ProvidersModel::new());

        let providers_list = Self { widget, builder, sender, model };
        providers_list.init();
        providers_list
    }

    fn init(&self) {
        let providers_container: gtk::Box = self.builder.get_object("providers_container").expect("Failed to retrieve providers_container");

        for (provider, accounts_model) in &self.model.borrow().model {
            if accounts_model.get_count() != 0 {
                let accounts_list = AccountsList::new(accounts_model, provider, self.sender.clone());
                providers_container.pack_start(&accounts_list.widget, false, false, 0);
            }
        }
    }
}
