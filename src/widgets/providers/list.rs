use crate::application::Action;
use crate::models::{Account, Provider, ProvidersModel};
use crate::widgets::{accounts::AccountRow, providers::ProviderRow};
use gio::ListModelExt;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct ProvidersList {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    sender: Sender<Action>,
    model: Rc<ProvidersModel>,
    filter_model: gtk::FilterListModel,
}

impl ProvidersList {
    pub fn new(model: Rc<ProvidersModel>, sender: Sender<Action>) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
        get_widget!(builder, gtk::Box, providers_box);
        let filter_model = gtk::FilterListModel::new(Some(&model.model), gtk::NONE_FILTER);
        let list = Self {
            widget: providers_box,
            builder,
            sender,
            model,
            filter_model,
        };
        list.init();
        list
    }

    pub fn search(&self, text: String) {
        get_widget!(self.builder, gtk::ListBox, providers_list);

        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(move |object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.search_accounts(text.clone());
            provider.accounts().get_n_items() != 0
        })));
        self.filter_model.set_filter(Some(&accounts_filter));
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::ListBox, providers_list);

        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(|object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        })));
        self.filter_model.set_filter(Some(&accounts_filter));

        providers_list.bind_model(
            Some(&self.filter_model),
            Some(Box::new(
                clone!(@strong self.sender as sender => move |obj| {
                    let provider = obj.downcast_ref::<Provider>().unwrap();
                    let row = ProviderRow::new(provider, sender.clone());
                    row.widget.upcast::<gtk::Widget>()
                }),
            )),
        );
    }
}
