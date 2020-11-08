use glib::Sender;
use gtk::prelude::*;

use crate::application::Action;
use crate::models::{Provider, ProvidersModel};

pub struct ProvidersList {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    sender: Sender<Action>,
}

impl ProvidersList {
    pub fn new(model: &ProvidersModel, sender: Sender<Action>) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/providers_list.ui");
        get_widget!(builder, gtk::Box, providers_box);

        let list = Self {
            widget: providers_box,
            builder,
            sender,
        };
        list.init(model);
        list
    }

    fn init(&self, model: &ProvidersModel) {
        get_widget!(self.builder, gtk::ListBox, providers_list);

        let accounts_filter = gtk::CustomFilter::new(Some(Box::new(|object| {
            let provider = object.downcast_ref::<Provider>().unwrap();
            provider.has_accounts()
        })));
        let providers_model = gtk::FilterListModel::new(Some(&model.model), Some(&accounts_filter));

        providers_list.bind_model(
            Some(&providers_model),
            Some(Box::new(move |obj| {
                let provider = obj.downcast_ref::<Provider>().unwrap();
                let row = ProviderRow::new(provider);
                row.widget.upcast::<gtk::Widget>()
            })),
        );
    }
}

pub struct ProviderRow<'a> {
    pub widget: gtk::ListBoxRow,
    provider: &'a Provider,
    builder: gtk::Builder,
}

impl<'a> ProviderRow<'a> {
    pub fn new(provider: &'a Provider) -> Self {
        let builder =
            gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/provider_row.ui");
        get_widget!(builder, gtk::ListBoxRow, provider_row);
        let row = Self {
            widget: provider_row,
            builder,
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
    }
}
