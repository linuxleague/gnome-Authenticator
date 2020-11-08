use crate::application::Action;
use crate::models::{Provider, ProvidersModel};
use crate::widgets::providers::ProviderRow;
use gio::ListModelExt;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct ProvidersList {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    pub filter_model: gtk::FilterListModel,
}

impl ProvidersList {
    pub fn new() -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
        get_widget!(builder, gtk::Box, providers_box);

        let filter_model = gtk::FilterListModel::new(gtk::NONE_FILTER_LIST_MODEL, gtk::NONE_FILTER);

        let list = Self {
            widget: providers_box,
            builder,
            filter_model,
        };
        list
    }

    pub fn set_model(&self, model: Rc<ProvidersModel>) {
        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(|object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        })));
        self.filter_model.set_filter(Some(&accounts_filter));
        self.filter_model.set_model(Some(&model.model));
    }

    pub fn refilter(&self) {
        if let Some(filter) = self.filter_model.get_filter() {
            filter.changed(gtk::FilterChange::Different);
        }
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

    pub fn init(&self, sender: Sender<Action>) {
        get_widget!(self.builder, gtk::ListBox, providers_list);

        providers_list.bind_model(
            Some(&self.filter_model),
            Some(Box::new(clone!(@strong sender => move |obj| {
                let provider = obj.downcast_ref::<Provider>().unwrap();
                let row = ProviderRow::new(provider, sender.clone());
                row.widget.upcast::<gtk::Widget>()
            }))),
        );
    }
}
