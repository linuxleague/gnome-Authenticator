use crate::application::Action;
use crate::helpers::qrcode;
use crate::models::database::*;
use crate::models::{Account, Algorithm, NewAccount, Provider, ProvidersModel};
use anyhow::Result;
use gio::prelude::*;
use glib::StaticType;
use glib::{signal::Inhibit, Sender};
use gtk::prelude::*;
use libhandy::ComboRowExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct AddAccountDialog {
    pub widget: libhandy::Window,
    builder: gtk::Builder,
    sender: Sender<Action>,
    model: Rc<ProvidersModel>,
    selected_provider: Rc<RefCell<Option<Provider>>>,
}

impl AddAccountDialog {
    pub fn new(sender: Sender<Action>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/add_account.ui");
        get_widget!(builder, libhandy::Window, add_dialog);

        let add_account_dialog = Rc::new(Self {
            widget: add_dialog,
            builder,
            sender,
            model: Rc::new(ProvidersModel::new()),
            selected_provider: Rc::new(RefCell::new(None)),
        });

        add_account_dialog.setup_actions(add_account_dialog.clone());
        add_account_dialog.setup_signals();
        add_account_dialog.setup_widgets(add_account_dialog.clone());
        add_account_dialog
    }

    fn add_account(&self, account: NewAccount) -> Result<Account> {
        // TODO: add the account to the provider model.
        account.insert()
    }

    fn setup_signals(&self) {
        get_widget!(self.builder, gtk::Entry, username_entry);
        get_widget!(self.builder, gtk::Entry, token_entry);

        //let action_group = self.widget.get_action_group("add").unwrap().downcast::<gio::SimpleActionGroup>().unwrap();
        //let save_action = action_group.lookup_action("save").unwrap().downcast::<gio::SimpleAction>().unwrap();

        let validate_entries = clone!(@weak username_entry, @weak token_entry => move |_: &gtk::Entry| {
            let username = username_entry.get_text().unwrap();
            let token = token_entry.get_text().unwrap();

            let is_valid = !(username.is_empty() || token.is_empty());
            //save_action.set_enabled(is_valid);
        });

        username_entry.connect_changed(validate_entries.clone());
        token_entry.connect_changed(validate_entries);
    }

    fn set_provider(&self, provider: Provider) {
        get_widget!(self.builder, gtk::Entry, @provider_entry).set_text(&provider.name());
        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton)
            .set_value(provider.period() as f64);

        if let Some(ref website) = provider.website() {
            get_widget!(self.builder, gtk::Entry, @provider_website_entry).set_text(website);
        }

        unsafe {
            // This is safe because of the repr(u32)
            let selected_position: u32 = std::mem::transmute(provider.algorithm());
            get_widget!(self.builder, libhandy::ComboRow, @algorithm_comborow)
                .set_selected(selected_position);
        }

        get_widget!(self.builder, gtk::Entry, @token_entry)
            .set_property_secondary_icon_sensitive(provider.help_url().is_some());

        self.selected_provider.replace(Some(provider));
    }

    fn setup_actions(&self, dialog: Rc<Self>) {
        let actions = gio::SimpleActionGroup::new();
        action!(
            actions,
            "back",
            clone!(@weak self.widget as dialog => move |_, _| {
                dialog.destroy();
            })
        );

        let save = gio::SimpleAction::new("save", None);
        save.connect_activate(clone!(@strong self.builder as builder => move |_, _| {
            get_widget!(builder, gtk::Entry, username_entry);
            get_widget!(builder, gtk::Entry, token_entry);
            get_widget!(builder, gtk::Entry, provider_entry);
            get_widget!(builder, gtk::Entry, website_entry);
            // get_widget!(builder, gtk::Entry, period_entry);
            // get_widget!(builder, gtk::Entry, algorithm_model);

            /*
            let new_account = NewAccount {
                username: username_entry.get_text().unwrap().to_string(),
                token_id: token_entry.get_text().unwrap().to_string(),
                provider: provider_combobox.get_active_id().unwrap().parse::<i32>().unwrap(),
            };
            if let Err(err) = add_account_dialog.add_account(new_account) {
                add_account_dialog.notify_err("Failed to add a new account");
            } else {
                // Close the dialog if everything is fine.
                add_account_dialog.widget.destroy();
            }
            */
        }));
        save.set_enabled(false);
        actions.add_action(&save);

        action!(
            actions,
            "scan-qr",
            clone!(@strong self.builder as builder, @strong dialog, @strong self.model as model => move |_, _| {
                qrcode::screenshot_area(clone!(@strong builder, @strong dialog, @strong model => move |screenshot| {
                    if let Ok(otpauth) = qrcode::scan(&gio::File::new_for_uri(&screenshot)) {
                        get_widget!(builder, gtk::Entry, @token_entry).set_text(&otpauth.token);
                        if let Some(ref username) = otpauth.account {
                            get_widget!(builder, gtk::Entry, @username_entry).set_text(&username);
                        }
                        if let Some(ref provider) = otpauth.issuer {
                            let provider = model.find_by_name(provider).unwrap();
                            dialog.set_provider(provider);
                        }
                    }
                }));
            })
        );
        self.widget.insert_action_group("add", Some(&actions));
    }

    fn setup_widgets(&self, dialog: Rc<Self>) {
        get_widget!(self.builder, gtk::EntryCompletion, provider_completion);
        provider_completion.set_model(Some(&self.model.completion_model()));

        get_widget!(self.builder, gtk::Entry, @token_entry)
            .set_property_secondary_icon_sensitive(false);

        get_widget!(self.builder, libhandy::ComboRow, algorithm_comborow);
        let algorithms_model = libhandy::EnumListModel::new(Algorithm::static_type());
        algorithm_comborow.set_model(Some(&algorithms_model));

        provider_completion.connect_match_selected(
            clone!(@strong dialog, @strong self.model as model => move |completion, store, iter| {
                let provider_id = store.get_value(iter, 0). get_some::<i32>().unwrap();
                let provider = model.find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );

        get_widget!(self.builder, gtk::Entry, token_entry);
        token_entry.connect_icon_press(clone!(@strong dialog => move |entry, pos| {
            if pos == gtk::EntryIconPosition::Secondary {
                if let Some(ref provider) = dialog.selected_provider.borrow().clone() {
                   gio::AppInfo::launch_default_for_uri(&provider.help_url().unwrap(),  None::<&gio::AppLaunchContext>);
                }
            }
        }));
        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton).set_value(30.0);
    }
}
