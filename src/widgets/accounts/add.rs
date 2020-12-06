use crate::application::Action;
use crate::helpers::{qrcode, Keyring};
use crate::models::{Account, Provider, ProvidersModel};
use anyhow::Result;
use gio::prelude::*;
use gio::{subclass::ObjectSubclass, ActionMapExt};
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use glib::{signal::Inhibit, Receiver, Sender};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::ActionRowExt;
use once_cell::sync::OnceCell;
use std::cell::RefCell;

pub enum AccountAddAction {
    SetIcon(gio::File),
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;

    #[derive(CompositeTemplate)]
    pub struct AccountAddDialog {
        pub global_sender: OnceCell<Sender<Action>>,
        pub sender: Sender<AccountAddAction>,
        pub receiver: RefCell<Option<Receiver<AccountAddAction>>>,
        pub model: OnceCell<ProvidersModel>,
        pub selected_provider: OnceCell<Provider>,
        pub actions: gio::SimpleActionGroup,

        #[template_child(id = "username_entry")]
        pub username_entry: TemplateChild<gtk::Entry>,

        #[template_child(id = "token_entry")]
        pub token_entry: TemplateChild<gtk::Entry>,

        #[template_child(id = "more_list")]
        pub more_list: TemplateChild<gtk::ListBox>,

        #[template_child(id = "period_label")]
        pub period_label: TemplateChild<gtk::Label>,

        #[template_child(id = "provider_entry")]
        pub provider_entry: TemplateChild<gtk::Entry>,

        #[template_child(id = "algorithm_label")]
        pub algorithm_label: TemplateChild<gtk::Label>,

        #[template_child(id = "provider_website_row")]
        pub provider_website_row: TemplateChild<libhandy::ActionRow>,

        #[template_child(id = "provider_help_row")]
        pub provider_help_row: TemplateChild<libhandy::ActionRow>,

        #[template_child(id = "provider_completion")]
        pub provider_completion: TemplateChild<gtk::EntryCompletion>,

        #[template_child(id = "image")]
        pub image: TemplateChild<gtk::Image>,

        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<gtk::Spinner>,

        #[template_child(id = "image_stack")]
        pub image_stack: TemplateChild<gtk::Stack>,
    }

    impl ObjectSubclass for AccountAddDialog {
        const NAME: &'static str = "AccountAddDialog";
        type Type = super::AccountAddDialog;
        type ParentType = libhandy::Window;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));

            let actions = gio::SimpleActionGroup::new();

            Self {
                global_sender: OnceCell::new(),
                sender,
                receiver,
                actions,
                model: OnceCell::new(),
                selected_provider: OnceCell::new(),
                token_entry: TemplateChild::default(),
                username_entry: TemplateChild::default(),
                more_list: TemplateChild::default(),
                period_label: TemplateChild::default(),
                provider_entry: TemplateChild::default(),
                algorithm_label: TemplateChild::default(),
                provider_website_row: TemplateChild::default(),
                provider_help_row: TemplateChild::default(),
                provider_completion: TemplateChild::default(),
                image: TemplateChild::default(),
                spinner: TemplateChild::default(),
                image_stack: TemplateChild::default(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/account_add.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for AccountAddDialog {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();
            self.parent_constructed(obj);
        }
    }
    impl WidgetImpl for AccountAddDialog {}
    impl WindowImpl for AccountAddDialog {}
    impl libhandy::subclass::window::WindowImpl for AccountAddDialog {}
}
glib_wrapper! {
    pub struct AccountAddDialog(ObjectSubclass<imp::AccountAddDialog>) @extends gtk::Widget, gtk::Window, libhandy::Window;
}

impl AccountAddDialog {
    pub fn new(model: ProvidersModel, global_sender: Sender<Action>) -> Self {
        let dialog = glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create AccountAddDialog")
            .downcast::<AccountAddDialog>()
            .expect("Created object is of wrong type");

        let self_ = imp::AccountAddDialog::from_instance(&dialog);
        self_.model.set(model).unwrap();
        self_.global_sender.set(global_sender).unwrap();

        dialog.setup_actions();
        dialog.setup_signals();
        dialog.setup_widgets();
        dialog
    }

    fn setup_signals(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);

        let username_entry = self_.username_entry.get();
        let token_entry = self_.token_entry.get();

        let validate_entries = clone!(@weak username_entry, @weak token_entry, @strong self_.actions as actions => move |_: &gtk::Entry| {
            let username = username_entry.get_text().unwrap();
            let token = token_entry.get_text().unwrap();

            let is_valid = !(username.is_empty() || token.is_empty());
            get_action!(actions, @save).set_enabled(is_valid);

        });

        username_entry.connect_changed(validate_entries.clone());
        token_entry.connect_changed(validate_entries);

        let event_controller = gtk::EventControllerKey::new();
        event_controller.connect_key_pressed(
            clone!(@weak self as widget => @default-return Inhibit(false), move |_, k, _, _| {
                if k == 65307 {
                    widget.close();
                }
                Inhibit(false)
            }),
        );
        self.add_controller(&event_controller);
    }

    fn scan_qr(&self) -> Result<()> {
        let self_ = imp::AccountAddDialog::from_instance(self);
        let token_entry = self_.token_entry.get();
        let username_entry = self_.username_entry.get();

        qrcode::screenshot_area(
            self.clone().upcast::<gtk::Window>(),
            clone!(@weak self as dialog, @weak token_entry, @weak username_entry, @strong self_.model as model,
                @strong self_.sender as sender => move |screenshot| {
                if let Ok(otpauth) = qrcode::scan(&screenshot) {
                    token_entry.set_text(&otpauth.token);
                    if let Some(ref username) = otpauth.account {
                        username_entry.set_text(&username);
                    }
                    if let Some(ref provider) = otpauth.issuer {
                        let provider = model.get().unwrap().find_by_name(provider).unwrap();
                        dialog.set_provider(provider);
                    }
                }
            }),
        )?;
        Ok(())
    }

    fn save(&self) -> Result<()> {
        let self_ = imp::AccountAddDialog::from_instance(self);

        if let Some(provider) = self_.selected_provider.get().clone() {
            let username = self_.username_entry.get().get_text().unwrap();
            let token = self_.token_entry.get().get_text().unwrap();

            if let Ok(token_id) = Keyring::store(&username, &token) {
                let account = Account::create(&username, &token_id, provider)?;
                send!(
                    self_.global_sender.get().unwrap(),
                    Action::AccountCreated(account, provider.clone())
                );
            }
            // TODO: display an error message saying there was an error form keyring
        }
        Ok(())
    }

    fn set_provider(&self, provider: Provider) {
        let self_ = imp::AccountAddDialog::from_instance(self);

        self_.more_list.get().show();

        self_.provider_entry.get().set_text(&provider.name());
        self_
            .period_label
            .get()
            .set_text(&provider.period().to_string());
        self_
            .algorithm_label
            .get()
            .set_text(&provider.algorithm().to_locale_string());

        if let Some(ref website) = provider.website() {
            self_.provider_website_row.get().set_subtitle(Some(website));
        }
        if let Some(ref help_url) = provider.help_url() {
            self_.provider_help_row.get().set_subtitle(Some(help_url));
        }

        self_.image_stack.get().set_visible_child_name("loading");
        self_.spinner.get().start();

        let p = provider.clone();
        let sender = self_.sender.clone();
        spawn!(async move {
            if let Ok(file) = p.favicon().await {
                send!(sender, AccountAddAction::SetIcon(file));
            }
        });

        self_.selected_provider.set(provider);
    }

    fn setup_actions(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        action!(
            self_.actions,
            "back",
            clone!(@weak self as dialog => move |_, _| {
                dialog.destroy();
            })
        );
        action!(
            self_.actions,
            "save",
            clone!(@weak self as dialog => move |_, _| {
                if dialog.save().is_ok() {
                    dialog.close();
                }
            })
        );

        action!(
            self_.actions,
            "scan-qr",
            clone!(@strong self as dialog => move |_, _| {
                dialog.scan_qr();
            })
        );
        self.insert_action_group("add", Some(&self_.actions));
        get_action!(self_.actions, @save).set_enabled(false);
    }

    fn setup_widgets(&self) {
        let self_ = imp::AccountAddDialog::from_instance(self);
        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as dialog => move |action| dialog.do_action(action)),
        );
        self_
            .provider_completion
            .get()
            .set_model(Some(&self_.model.get().unwrap().completion_model()));
        self_.provider_completion.get().connect_match_selected(
            clone!(@strong self as dialog, @strong self_.model as model => move |_, store, iter| {
                let provider_id = store.get_value(iter, 0). get_some::<i32>().unwrap();
                let provider = model.get().unwrap().find_by_id(provider_id).unwrap();
                dialog.set_provider(provider);

                Inhibit(false)
            }),
        );
    }

    fn do_action(&self, action: AccountAddAction) -> glib::Continue {
        match action {
            AccountAddAction::SetIcon(file) => {
                let self_ = imp::AccountAddDialog::from_instance(self);
                self_.image.get().set_from_file(file.get_path().unwrap());
                self_.spinner.get().stop();
                self_.image_stack.get().set_visible_child_name("image");
            }
        };
        glib::Continue(true)
    }
}
