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
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
        get_widget!(builder, gtk::Box, providers_list);
        let model = RefCell::new(ProvidersModel::new());

        let list = Self {
            widget: providers_list,
            builder,
            sender,
            model,
        };
        list.init();
        list
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::Box, providers_container);
        /*
        for (provider, accounts_model) in &self.model.borrow().model {
            if accounts_model.get_count() != 0 {
                let accounts_list =
                    AccountsList::new(accounts_model, provider, self.sender.clone());
                providers_container.append(&accounts_list.widget);
            }
        }
        */
    }
}
