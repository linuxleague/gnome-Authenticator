use crate::application::Application;
use crate::config;
use crate::helpers::Keyring;
use crate::models::{Account, Provider, ProvidersModel};
use crate::widgets::{accounts::QRCodePage, providers::ProvidersList, AccountAddDialog};
use crate::window_state;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::{clone, glib_object_subclass, glib_wrapper};
use glib::{signal::Inhibit, Receiver, Sender};
use gtk::{prelude::*, CompositeTemplate};
use gtk_macros::action;
use once_cell::sync::OnceCell;

#[derive(PartialEq, Debug)]
pub enum View {
    Login,
    Accounts,
    Account(Account),
}

pub enum Action {
    AccountCreated(Account, Provider),
    OpenAddAccountDialog,
    SetView(View),
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::application_window::ApplicationWindowImpl as HdyApplicationWindowImpl;
    use std::cell::RefCell;

    #[derive(Debug, CompositeTemplate)]
    pub struct Window {
        pub sender: Sender<Action>,
        pub receiver: RefCell<Option<Receiver<Action>>>,
        pub settings: gio::Settings,
        pub providers: ProvidersList,
        pub qrcode_page: QRCodePage,
        pub model: OnceCell<ProvidersModel>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub deck: TemplateChild<libhandy::Leaflet>,
        #[template_child]
        pub container: TemplateChild<gtk::Box>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub locked_img: TemplateChild<gtk::Image>,
    }

    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = libhandy::ApplicationWindow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(config::APP_ID);
            let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let receiver = RefCell::new(Some(r));
            let providers = ProvidersList::new(sender.clone());
            Self {
                settings,
                providers,
                sender,
                receiver,
                model: OnceCell::new(),
                qrcode_page: QRCodePage::new(),
                search_entry: TemplateChild::default(),
                deck: TemplateChild::default(),
                container: TemplateChild::default(),
                search_bar: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                password_entry: TemplateChild::default(),
                locked_img: TemplateChild::default(),
            }
        }
        fn class_init(klass: &mut Self::Class) {
            klass.set_template_from_resource("/com/belmoussaoui/Authenticator/window.ui");
            Self::bind_template_children(klass);
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self, obj: &Self::Type) {
            obj.init_template();

            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl HdyApplicationWindowImpl for Window {}
}

glib_wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, libhandy::ApplicationWindow, gio::ActionMap;
}

impl Window {
    pub fn new(model: ProvidersModel, app: &Application) -> Self {
        let window = glib::Object::new(Window::static_type(), &[("application", app)])
            .unwrap()
            .downcast::<Window>()
            .unwrap();
        app.add_window(&window);

        if config::PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(model);
        window.setup_actions(app);
        window.set_view(View::Login); // Start by default in an empty state
        window.setup_signals(app);
        window
    }

    pub fn set_view(&self, view: View) {
        let self_ = imp::Window::from_instance(self);
        match view {
            View::Login => {
                self_.deck.get().set_visible_child_name("login");
            }
            View::Accounts => {
                self_.deck.get().set_visible_child_name("accounts");
            }
            View::Account(account) => {
                self_.deck.get().set_visible_child_name("account");
                self_.qrcode_page.set_account(&account);
            }
        }
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        let self_ = imp::Window::from_instance(self);

        match action {
            Action::OpenAddAccountDialog => {
                let model = self_.model.get().unwrap();

                let dialog = AccountAddDialog::new(model.clone(), self_.sender.clone());
                dialog.set_transient_for(Some(self));
                dialog.show();
            }
            Action::AccountCreated(account, provider) => {
                let model = self_.model.get().unwrap();
                model.add_account(&account, &provider);
                self.providers().refilter();
            }
            Action::SetView(view) => {
                self.set_view(view);
            }
        };

        glib::Continue(true)
    }

    pub fn providers(&self) -> ProvidersList {
        let self_ = imp::Window::from_instance(self);
        self_.providers.clone()
    }

    fn init(&self, model: ProvidersModel) {
        let self_ = imp::Window::from_instance(self);
        self_.model.set(model.clone());
        self_.providers.set_model(model.clone());

        self.set_icon_name(Some(config::APP_ID));
        self_
            .locked_img
            .get()
            .set_from_icon_name(Some(config::APP_ID));

        // load latest window state
        window_state::load(&self, &self_.settings);
        // save window state on delete event
        self.connect_close_request(clone!(@weak self_.settings as settings => move |window| {
            if let Err(err) = window_state::save(&window, &settings) {
                warn!("Failed to save window state {:#?}", err);
            }
            Inhibit(false)
        }));

        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/shortcuts.ui");
        gtk_macros::get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.set_help_overlay(Some(&shortcuts));

        self_.container.get().append(&self_.providers);

        let page = self_.deck.get().add(&self_.qrcode_page).unwrap();
        page.set_name("account");

        self_
            .search_btn
            .get()
            .bind_property("active", &self_.search_bar.get(), "search-mode-enabled")
            .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
            .build();

        self_.search_entry.get().connect_search_changed(
            clone!(@weak self_.providers as providers => move |entry| {
                let text = entry.get_text().unwrap().to_string();
                providers.search(text);
            }),
        );

        let search_btn = self_.search_btn.get();
        self_
            .search_entry
            .get()
            .connect_stop_search(clone!(@weak search_btn => move |entry| {
                entry.set_text("");
                search_btn.set_active(false);
            }));

        let gtk_settings = gtk::Settings::get_default().unwrap();
        self_.settings.bind(
            "dark-theme",
            &gtk_settings,
            "gtk-application-prefer-dark-theme",
            gio::SettingsBindFlags::DEFAULT,
        );

        let receiver = self_.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@weak self as win => move |action| win.do_action(action)),
        );
    }

    fn setup_actions(&self, app: &Application) {
        let self_ = imp::Window::from_instance(self);
        let search_btn = self_.search_btn.get();
        action!(
            self,
            "search",
            clone!(@weak search_btn => move |_,_| {
                search_btn.set_active(!search_btn.get_active());
            })
        );

        action!(
            self,
            "back",
            clone!(@weak self as win => move |_, _| {
                // Always return back to accounts list
                win.set_view(View::Accounts);
            })
        );

        action!(
            self,
            "add-account",
            clone!(@strong self_.sender as sender => move |_,_| {
                gtk_macros::send!(sender, Action::OpenAddAccountDialog);
            })
        );

        let password_entry = self_.password_entry.get();
        action!(
            self,
            "unlock",
            clone!(@strong self_.sender as sender, @weak password_entry, @weak app => move |_, _| {
                let password = password_entry.get_text().unwrap();
                if Keyring::is_current_password(&password).unwrap() {
                    password_entry.set_text("");
                    app.set_locked(false);
                    gtk_macros::send!(sender, Action::SetView(View::Accounts));
                }
            })
        );
    }

    fn setup_signals(&self, app: &Application) {
        app.connect_local(
            "notify::locked",
            false,
            clone!(@weak app, @weak self as win => move |_| {
                if app.locked(){
                    win.set_view(View::Login);
                } else {
                    win.set_view(View::Accounts);
                };
                None
            }),
        )
        .unwrap();
    }
}
