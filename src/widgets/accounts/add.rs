use crate::application::Action;
use crate::helpers::qrcode;
use crate::models::{Account, Provider, ProvidersModel};
use anyhow::Result;
use gio::prelude::*;
use glib::{signal::Inhibit, Receiver, Sender};
use gtk::prelude::*;
use libhandy::ActionRowExt;
use std::cell::RefCell;
use std::rc::Rc;

pub enum AddAccountAction {
    SetIcon(gio::File),
    SetProvider(Provider),
    Save,
    ScanQR,
}

pub struct AddAccountDialog {
    pub widget: libhandy::Window,
    builder: gtk::Builder,
    global_sender: Sender<Action>,
    sender: Sender<AddAccountAction>,
    receiver: RefCell<Option<Receiver<AddAccountAction>>>,
    model: Rc<ProvidersModel>,
    selected_provider: Rc<RefCell<Option<Provider>>>,
    actions: gio::SimpleActionGroup,
}

impl AddAccountDialog {
    pub fn new(model: Rc<ProvidersModel>, global_sender: Sender<Action>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/account_add.ui");
        get_widget!(builder, libhandy::Window, add_dialog);

        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));
        let actions = gio::SimpleActionGroup::new();

        let add_account_dialog = Rc::new(Self {
            widget: add_dialog,
            builder,
            global_sender,
            sender,
            receiver,
            actions,
            model,
            selected_provider: Rc::new(RefCell::new(None)),
        });

        add_account_dialog.setup_actions();
        add_account_dialog.setup_signals();
        add_account_dialog.setup_widgets(add_account_dialog.clone());
        add_account_dialog
    }

    fn setup_signals(&self) {
        get_widget!(self.builder, gtk::Entry, username_entry);
        get_widget!(self.builder, gtk::Entry, token_entry);

        let validate_entries = clone!(@weak username_entry, @weak token_entry, @strong self.actions as actions => move |_: &gtk::Entry| {
            let username = username_entry.get_text().unwrap();
            let token = token_entry.get_text().unwrap();

            let is_valid = !(username.is_empty() || token.is_empty());
            get_action!(actions, @save).set_enabled(is_valid);

        });

        username_entry.connect_changed(validate_entries.clone());
        token_entry.connect_changed(validate_entries);

        let event_controller = gtk::EventControllerKey::new();
        event_controller.connect_key_pressed(clone!(@weak self.widget as widget => @default-return Inhibit(false), move |_, k, _, _| {
            if k == 65307 {
                widget.close();
            }
            Inhibit(false)
        }));
        self.widget.add_controller(&event_controller);
    }

    fn scan_qr(&self) -> Result<()> {
        qrcode::screenshot_area(
            clone!(@strong self.builder as builder, @strong self.model as model,
                @strong self.sender as sender => move |screenshot| {
                if let Ok(otpauth) = qrcode::scan(&gio::File::new_for_uri(&screenshot)) {
                    get_widget!(builder, gtk::Entry, @token_entry).set_text(&otpauth.token);
                    if let Some(ref username) = otpauth.account {
                        get_widget!(builder, gtk::Entry, @username_entry).set_text(&username);
                    }
                    if let Some(ref provider) = otpauth.issuer {
                        let provider = model.find_by_name(provider).unwrap();
                        send!(sender, AddAccountAction::SetProvider(provider));
                    }
                }
            }),
        )?;
        Ok(())
    }

    fn save(&self) -> Result<()> {
        if let Some(provider) = self.selected_provider.borrow().clone() {
            let username = get_widget!(self.builder, gtk::Entry, @username_entry)
                .get_text()
                .unwrap();
            let token = get_widget!(self.builder, gtk::Entry, @token_entry)
                .get_text()
                .unwrap();

            let account = Account::create(&username, &token, provider.id())?;
            send!(
                self.global_sender,
                Action::AccountCreated(account, provider)
            );
        }
        Ok(())
    }

    fn set_provider(&self, provider: Provider) {
        get_widget!(self.builder, gtk::ListBox, @more_list).show();
        get_widget!(self.builder, gtk::Entry, @provider_entry).set_text(&provider.name());
        get_widget!(self.builder, gtk::Label, @period_label)
            .set_text(&format!("{} seconds", provider.period()));
        get_widget!(self.builder, gtk::Label, @algorithm_label)
            .set_text(&provider.algorithm().to_locale_string());

        if let Some(ref website) = provider.website() {
            get_widget!(self.builder, libhandy::ActionRow, provider_website_row);
            provider_website_row.set_subtitle(Some(website));
        }
        if let Some(ref help_url) = provider.help_url() {
            get_widget!(self.builder, libhandy::ActionRow, provider_help_row);
            provider_help_row.set_subtitle(Some(help_url));
        }

        get_widget!(self.builder, gtk::Stack, @image_stack).set_visible_child_name("loading");
        get_widget!(self.builder, gtk::Spinner, @spinner).start();

        let p = provider.clone();
        let sender = self.sender.clone();
        spawn!(async move {
            if let Ok(file) = p.favicon().await {
                send!(sender, AddAccountAction::SetIcon(file));
            }
        });

        self.selected_provider.replace(Some(provider));
    }

    fn setup_actions(&self) {
        action!(
            self.actions,
            "back",
            clone!(@weak self.widget as dialog => move |_, _| {
                dialog.destroy();
            })
        );

        action!(
            self.actions,
            "save",
            clone!(@strong self.sender as sender => move |_, _| {
                send!(sender, AddAccountAction::Save);
            })
        );

        action!(
            self.actions,
            "scan-qr",
            clone!(@strong self.sender as sender => move |_, _| {
                send!(sender, AddAccountAction::ScanQR);
            })
        );
        self.widget.insert_action_group("add", Some(&self.actions));
        get_action!(self.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self, dialog: Rc<Self>) {
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong dialog => move |action| dialog.do_action(action)),
        );

        get_widget!(self.builder, gtk::EntryCompletion, provider_completion);
        provider_completion.set_model(Some(&self.model.completion_model()));

        get_widget!(self.builder, gtk::Entry, @token_entry);

        provider_completion.connect_match_selected(
            clone!(@strong dialog, @strong self.model as model => move |_, store, iter| {
                let provider_id = store.get_value(iter, 0). get_some::<i32>().unwrap();
                let provider = model.find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );
    }

    fn do_action(&self, action: AddAccountAction) -> glib::Continue {
        match action {
            AddAccountAction::SetIcon(file) => {
                get_widget!(self.builder, gtk::Image, @image)
                    .set_from_file(file.get_path().unwrap());
                get_widget!(self.builder, gtk::Spinner, @spinner).stop();
                get_widget!(self.builder, gtk::Stack, @image_stack).set_visible_child_name("image");
            }
            AddAccountAction::SetProvider(p) => self.set_provider(p),
            AddAccountAction::Save => {
                if self.save().is_ok() {
                    self.widget.close();
                }
            }
            AddAccountAction::ScanQR => {
                self.scan_qr();
            }
        };
        glib::Continue(true)
    }
}
