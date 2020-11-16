use super::window::PreferencesAction;
use crate::models::{Provider, ProvidersModel};
use gio::ListModelExt;
use glib::Sender;
use gtk::prelude::*;
use std::rc::Rc;

pub struct ProvidersPage {
    pub widget: libhandy::PreferencesPage,
    builder: gtk::Builder,
    model: Rc<ProvidersModel>,
    sender: Sender<PreferencesAction>,
}

impl ProvidersPage {
    pub fn new(model: Rc<ProvidersModel>, sender: Sender<PreferencesAction>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource(
            "/com/belmoussaoui/Authenticator/preferences_providers_page.ui",
        );
        get_widget!(builder, libhandy::PreferencesPage, providers_page);

        let page = Rc::new(Self {
            widget: providers_page,
            builder,
            model,
            sender,
        });
        page.init();
        page
    }

    fn init(&self) {
        get_widget!(self.builder, gtk::ListView, providers_list);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_bind(|_, list_item| {
            let item = list_item.get_item().unwrap();
            let provider = item.downcast_ref::<Provider>().unwrap();
            let row = Row::new(provider);
            list_item.set_child(Some(&row.widget));
        });
        providers_list.set_factory(Some(&factory));

        let selection_model = gtk::NoSelection::new(Some(&self.model.model));
        providers_list.set_model(Some(&selection_model));

        providers_list.connect_activate(
            clone!(@strong self.sender as sender => move|listview, pos|{
                let model = listview.get_model().unwrap();
                let provider = model.get_object(pos).unwrap().downcast::<Provider>().unwrap();
                send!(sender, PreferencesAction::EditProvider(provider));
            }),
        );
    }
}

pub struct Row<'a> {
    pub widget: libhandy::ActionRow,
    provider: &'a Provider,
}

impl<'a> Row<'a> {
    pub fn new(provider: &'a Provider) -> Self {
        let widget = libhandy::ActionRowBuilder::new()
            .title(&provider.name())
            .build();
        Self { widget, provider }
    }
}
