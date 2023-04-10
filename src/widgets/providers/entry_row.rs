use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone},
    Inhibit,
};

use crate::models::{Provider, ProvidersModel};

mod imp {
    use std::cell::RefCell;

    use glib::{subclass::Signal, translate::*};
    use once_cell::sync::Lazy;

    extern "C" {
        fn gtk_list_item_set_focusable(
            item: *mut gtk::ffi::GtkListItem,
            focusable: glib::ffi::gboolean,
        );
    }

    use super::*;
    use crate::models::ProviderSorter;

    #[derive(Debug, Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(resource = "/com/belmoussaoui/Authenticator/provider_entry_row.ui")]
    #[properties(wrapper_type = super::ProviderEntryRow)]
    pub struct ProviderEntryRow {
        #[template_child]
        pub popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        pub filter_model: gtk::FilterListModel,
        #[property(get, set = Self::set_provider, nullable)]
        pub provider: RefCell<Option<Provider>>,
        #[property(get, set = Self::set_model)]
        pub model: RefCell<Option<ProvidersModel>>,
        pub selection_model: gtk::SingleSelection,
        pub changed_handler: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProviderEntryRow {
        const NAME: &'static str = "ProviderEntryRow";
        type Type = super::ProviderEntryRow;
        type ParentType = adw::EntryRow;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action("entry.create", None, |widget, _, _| {
                widget.imp().popover.popdown();
                widget.emit_by_name::<()>("create", &[]);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProviderEntryRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            if let Some(value) = self.delegate_get_property(id, pspec) {
                value
            } else {
                self.derived_property(id, pspec)
            }
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            if !self.delegate_set_property(id, value, pspec) {
                self.derived_set_property(id, value, pspec)
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("create").action().build()]);
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.popover.set_parent(&*obj);

            let handler_id = obj.connect_changed(move |entry| {
                entry.on_changed();
            });
            self.changed_handler.replace(Some(handler_id));

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                unsafe {
                    gtk_list_item_set_focusable(item.as_ptr(), true.into_glib());
                }
                item.set_activatable(true);
                item.set_selectable(true);
                let row = gtk::Label::builder()
                    .halign(gtk::Align::Start)
                    .focusable(true)
                    .build();
                item.set_child(Some(&row));
            });
            factory.connect_bind(move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let provider = item.item().and_downcast::<Provider>().unwrap();
                let child = item.child().and_downcast::<gtk::Label>().unwrap();
                child.set_label(&provider.name());
            });
            self.list_view.set_factory(Some(&factory));

            let sorter = ProviderSorter::default();
            let property_expression = Provider::this_expression("name");

            let filter = gtk::StringFilter::new(Some(&property_expression));
            self.filter_model.set_filter(Some(&filter));
            let stack = self.stack.get();
            let popover = self.popover.get();
            self.selection_model.connect_items_changed(
                clone!(@weak stack, @weak popover => move |model, _, _ ,_| {
                    if model.n_items() == 0 {
                        stack.set_visible_child_name("empty");
                        popover.remove_css_class("menu");
                    } else {
                        popover.add_css_class("menu");
                        stack.set_visible_child_name("list");
                    }
                }),
            );

            let sorted_model =
                gtk::SortListModel::new(Some(self.filter_model.clone()), Some(sorter));
            self.selection_model.set_model(Some(&sorted_model));
            self.selection_model.set_autoselect(false);
            self.list_view.set_model(Some(&self.selection_model));
        }

        fn dispose(&self) {
            self.popover.unparent();
        }
    }

    impl WidgetImpl for ProviderEntryRow {
        fn focus(&self, dir: gtk::DirectionType) -> bool {
            if self.popover.is_visible() && dir == gtk::DirectionType::TabForward
                || dir == gtk::DirectionType::TabBackward
            {
                let matches = self.selection_model.n_items();
                let mut selected = self.selection_model.selected();

                if dir == gtk::DirectionType::TabForward {
                    if selected == gtk::INVALID_LIST_POSITION || selected == matches - 1 {
                        selected = 0;
                    } else {
                        selected += 1;
                    }
                } else {
                    if selected == gtk::INVALID_LIST_POSITION || selected == 0 {
                        selected = matches - 1;
                    } else {
                        selected -= 1;
                    }
                }
                self.selection_model.set_selected(selected);
                let item = self.selection_model.selected_item();
                self.obj()
                    .update_selected_provider(item.and_downcast_ref::<Provider>());
                return true;
            }
            self.parent_focus(dir)
        }
    }
    impl ListBoxRowImpl for ProviderEntryRow {}
    impl PreferencesRowImpl for ProviderEntryRow {}
    impl EntryRowImpl for ProviderEntryRow {}
    impl EditableImpl for ProviderEntryRow {}
    impl ProviderEntryRow {
        fn set_model(&self, model: &ProvidersModel) {
            self.filter_model.set_model(Some(model));
            self.model.replace(Some(model.clone()));
        }

        fn set_provider(&self, item: Option<Provider>) {
            println!("setting provider");
            // Do nothing if it is the already set provider
            let obj = self.obj();
            if item == obj.provider() {
                return;
            }
            obj.update_selected_provider(item.as_ref());
            let guard = obj.freeze_notify();
            self.provider.replace(item);
            drop(guard);
        }
    }
}

glib::wrapper! {
    pub struct ProviderEntryRow(ObjectSubclass<imp::ProviderEntryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Editable;
}

#[gtk::template_callbacks]
impl ProviderEntryRow {
    #[template_callback]
    fn on_changed(&self) {
        let imp = self.imp();
        let text = self.text();
        let filter = imp
            .filter_model
            .filter()
            .and_downcast::<gtk::StringFilter>()
            .unwrap();
        imp.popover.popup();
        filter.set_search(Some(&text));
    }

    #[template_callback]
    fn on_provider_activated(&self, position: u32) {
        println!("activated");
        let imp = self.imp();
        let item = imp.selection_model.item(position);
        imp.popover.popdown();
        self.set_provider(item.and_downcast::<Provider>());
    }

    // Re-implementation of ephy-location-entry.c
    #[template_callback]
    fn on_key_pressed(
        &self,
        keyval: gdk::Key,
        _keycode: u32,
        modifier: gdk::ModifierType,
    ) -> Inhibit {
        println!("keypress");
        let imp = self.imp();
        const PAGE_STEP: u32 = 20;
        if !modifier.is_empty() {
            return Inhibit(false);
        }
        if ![
            gdk::Key::Up,
            gdk::Key::KP_Up,
            gdk::Key::Down,
            gdk::Key::KP_Down,
            gdk::Key::Page_Up,
            gdk::Key::KP_Page_Up,
            gdk::Key::Page_Down,
            gdk::Key::KP_Page_Down,
        ]
        .contains(&keyval)
        {
            return Inhibit(false);
        }

        if !imp.popover.is_visible() {
            return Inhibit(false);
        }
        let matches = imp.selection_model.n_items();
        let mut selected = imp.selection_model.selected();

        if keyval == gdk::Key::Up || keyval == gdk::Key::KP_Up {
            if selected == gtk::INVALID_LIST_POSITION {
                selected = matches - 1;
            } else if selected == 0 {
                selected = gtk::INVALID_LIST_POSITION;
            } else {
                selected -= 1;
            }
        }
        if keyval == gdk::Key::Down || keyval == gdk::Key::KP_Down {
            if selected == gtk::INVALID_LIST_POSITION {
                selected = 0;
            } else if selected == matches - 1 {
                selected = gtk::INVALID_LIST_POSITION;
            } else {
                selected += 1;
            }
        }
        if keyval == gdk::Key::Page_Up || keyval == gdk::Key::KP_Page_Up {
            if selected == gtk::INVALID_LIST_POSITION {
                selected = matches - 1;
            } else if selected == 0 {
                selected = gtk::INVALID_LIST_POSITION;
            } else if selected < PAGE_STEP {
                selected = 0;
            } else {
                selected -= PAGE_STEP;
            }
        }
        if keyval == gdk::Key::Page_Down || keyval == gdk::Key::KP_Page_Down {
            if selected == gtk::INVALID_LIST_POSITION {
                selected = 0;
            } else if selected == matches - 1 {
                selected = gtk::INVALID_LIST_POSITION;
            } else if (selected + PAGE_STEP) > matches - 1 {
                selected = matches - 1;
            } else {
                selected += PAGE_STEP;
            }
        }

        if selected == gtk::INVALID_LIST_POSITION {
            self.error_bell();
            return Inhibit(true);
        }
        imp.selection_model.set_selected(selected);
        let item = imp.selection_model.selected_item();
        self.update_selected_provider(item.and_downcast_ref::<Provider>());
        Inhibit(true)
    }

    fn update_selected_provider(&self, provider: Option<&Provider>) {
        let imp = self.imp();
        let handler_id = imp.changed_handler.borrow();
        self.block_signal(handler_id.as_ref().unwrap());
        if let Some(item) = provider {
            self.set_text(&item.name());
        } else {
            self.set_text("");
        }
        self.set_position(-1);
        self.unblock_signal(handler_id.as_ref().unwrap());
    }
}
