use crate::application::Action;
use crate::helpers::qrcode;
use crate::models::{Account, Algorithm, Provider, ProvidersModel};
use anyhow::Result;
use gio::prelude::*;
use glib::StaticType;
use glib::{signal::Inhibit, Receiver, Sender};
use gtk::prelude::*;
use libhandy::ComboRowExt;
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
    pub fn new(global_sender: Sender<Action>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/add_account.ui");
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
            model: Rc::new(ProvidersModel::new()),
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
    }

    fn scan_qr(&self) {
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
        )
        .unwrap();
    }

    fn save(&self) -> Result<()> {
        let provider = match self.selected_provider.borrow().clone() {
            Some(p) => p,
            None => {
                let provider_website =
                    get_widget!(self.builder, gtk::Entry, @provider_website_entry).get_text();
                let provider_name = get_widget!(self.builder, gtk::Entry, @provider_entry)
                    .get_text()
                    .unwrap();
                let period = get_widget!(self.builder, gtk::SpinButton, @period_spinbutton)
                    .get_value() as i32;

                let selected_alg =
                    get_widget!(self.builder, libhandy::ComboRow, @algorithm_comborow)
                        .get_selected();
                let algorithm: Algorithm = unsafe { std::mem::transmute(selected_alg) };
                Provider::create(
                    &provider_name,
                    period,
                    algorithm,
                    provider_website.map(|w| w.to_string()),
                )
                .unwrap()
            }
        };
        let username = get_widget!(self.builder, gtk::Entry, @username_entry)
            .get_text()
            .unwrap();
        let token = get_widget!(self.builder, gtk::Entry, @token_entry)
            .get_text()
            .unwrap();

        let account = Account::create(&username, &token, provider.id())?;
        send!(self.global_sender, Action::AccountCreated(account));
        Ok(())
    }

    fn set_provider(&self, provider: Provider) {
        get_widget!(self.builder, gtk::Entry, @provider_entry).set_text(&provider.name());
        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton)
            .set_value(provider.period() as f64);

        if let Some(ref website) = provider.website() {
            get_widget!(self.builder, gtk::Entry, @provider_website_entry).set_text(website);
        }

        get_widget!(self.builder, gtk::Stack, @image_stack).set_visible_child_name("loading");
        get_widget!(self.builder, gtk::Spinner, @spinner).start();

        unsafe {
            // This is safe because of the repr(u32)
            let selected_position: u32 = std::mem::transmute(provider.algorithm());
            get_widget!(self.builder, libhandy::ComboRow, @algorithm_comborow)
                .set_selected(selected_position);
        }
        let p = provider.clone();
        let sender = self.sender.clone();
        spawn!(async move {
            if let Ok(file) = p.favicon().await {
                send!(sender, AddAccountAction::SetIcon(file));
            }
        });

        get_widget!(self.builder, gtk::Entry, @token_entry)
            .set_property_secondary_icon_sensitive(provider.help_url().is_some());

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

        get_widget!(self.builder, gtk::Entry, @token_entry)
            .set_property_secondary_icon_sensitive(false);

        get_widget!(self.builder, libhandy::ComboRow, algorithm_comborow);
        let algorithms_model = libhandy::EnumListModel::new(Algorithm::static_type());
        algorithm_comborow.set_model(Some(&algorithms_model));

        provider_completion.connect_match_selected(
            clone!(@strong dialog, @strong self.model as model => move |_, store, iter| {
                let provider_id = store.get_value(iter, 0). get_some::<i32>().unwrap();
                let provider = model.find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );

        get_widget!(self.builder, gtk::Entry, token_entry);
        token_entry.connect_icon_press(clone!(@strong dialog => move |_, pos| {
            if pos == gtk::EntryIconPosition::Secondary {
                if let Some(ref provider) = dialog.selected_provider.borrow().clone() {
                   provider.open_help();
                }
            }
        }));
        get_widget!(self.builder, gtk::SpinButton, @period_spinbutton).set_value(30.0);
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
                self.save().unwrap();
            }
            AddAccountAction::ScanQR => self.scan_qr(),
        };
        glib::Continue(true)
    }
}
