use crate::models::{Algorithm, Provider};
use glib::translate::ToGlib;
use gtk::prelude::*;
use libhandy::{ComboRowExt, EnumListModelExt};
use std::rc::Rc;

pub struct ProviderPage {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    algorithms_model: libhandy::EnumListModel,
}

impl ProviderPage {
    pub fn new() -> Rc<Self> {
        let builder = gtk::Builder::from_resource(
            "/com/belmoussaoui/Authenticator/preferences_provider_page.ui",
        );
        get_widget!(builder, gtk::Box, provider_page);
        let algorithms_model = libhandy::EnumListModel::new(Algorithm::static_type());

        let page = Rc::new(Self {
            widget: provider_page,
            builder,
            algorithms_model,
        });
        page.init();
        page
    }

    pub fn set_provider(&self, provider: Provider) {
        get_widget!(self.builder, gtk::Entry, @name_entry).set_text(&provider.name());
        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton)
            .set_value(provider.period() as f64);

        if let Some(ref website) = provider.website() {
            get_widget!(self.builder, gtk::Entry, @provider_website_entry).set_text(website);
        }

        get_widget!(self.builder, gtk::Stack, @image_stack).set_visible_child_name("loading");
        get_widget!(self.builder, gtk::Spinner, @spinner).start();

        get_widget!(self.builder, libhandy::ComboRow, algorithm_comborow);
        algorithm_comborow.set_selected(
            self.algorithms_model
                .find_position(provider.algorithm().to_glib()),
        );

        /*let sender = self.sender.clone();
        spawn!(async move {
            if let Ok(file) = p.favicon().await {
                send!(sender, AddAccountAction::SetIcon(file));
            }
        });*/

        get_widget!(self.builder, gtk::Label, title);
        title.set_text(&format!("Editing provider: {}", provider.name()));
    }

    fn init(&self) {
        get_widget!(self.builder, libhandy::ComboRow, algorithm_comborow);
        algorithm_comborow.set_model(Some(&self.algorithms_model));
    }
}
