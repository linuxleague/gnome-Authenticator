use crate::application::{Action, Application};
use crate::config::{APP_ID, PROFILE};
use crate::helpers::Keyring;
use crate::models::ProvidersModel;
use crate::widgets::providers::ProvidersList;
use crate::window_state;
use gio::prelude::*;
use gio::subclass::ObjectSubclass;
use glib::subclass::prelude::*;
use glib::{glib_object_subclass, glib_wrapper};
use glib::{signal::Inhibit, Sender};
use gtk::{prelude::*, CompositeTemplate};
use libhandy::prelude::*;

#[derive(PartialEq, Debug)]
pub enum View {
    Login,
    Accounts,
}

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::subclass::prelude::*;
    use libhandy::subclass::application_window::ApplicationWindowImpl as HdyApplicationWindowImpl;

    #[derive(Debug, CompositeTemplate)]
    pub struct Window {
        pub settings: gio::Settings,
        pub providers: ProvidersList,
        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child(id = "deck")]
        pub deck: TemplateChild<libhandy::Leaflet>,
        #[template_child(id = "container")]
        pub container: TemplateChild<gtk::Box>,
        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child(id = "search_btn")]
        pub search_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child(id = "password_entry")]
        pub password_entry: TemplateChild<gtk::PasswordEntry>,
    }

    impl ObjectSubclass for Window {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = libhandy::ApplicationWindow;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib_object_subclass!();

        fn new() -> Self {
            let settings = gio::Settings::new(APP_ID);
            let providers = ProvidersList::new();
            Self {
                settings,
                providers,
                search_entry: TemplateChild::default(),
                deck: TemplateChild::default(),
                container: TemplateChild::default(),
                search_bar: TemplateChild::default(),
                search_btn: TemplateChild::default(),
                password_entry: TemplateChild::default(),
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
    pub fn new(model: ProvidersModel, sender: Sender<Action>, app: &Application) -> Self {
        let window = glib::Object::new(Window::static_type(), &[("application", app)])
            .unwrap()
            .downcast::<Window>()
            .unwrap();
        app.add_window(&window);

        if PROFILE == "Devel" {
            window.get_style_context().add_class("devel");
        }
        window.init(model);
        window.setup_actions(app, sender.clone());
        window.set_view(View::Login); // Start by default in an empty state
        window.setup_signals(app, sender);
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
        }
    }

    pub fn providers(&self) -> ProvidersList {
        let self_ = imp::Window::from_instance(self);
        self_.providers.clone()
    }

    fn init(&self, model: ProvidersModel) {
        let self_ = imp::Window::from_instance(self);
        self_.providers.set_model(model.clone());
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
        get_widget!(builder, gtk::ShortcutsWindow, shortcuts);
        self.set_help_overlay(Some(&shortcuts));

        self_.container.get().append(&self_.providers);

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
    }

    fn setup_actions(&self, app: &Application, sender: Sender<Action>) {
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
            "add-account",
            clone!(@strong sender => move |_,_| {
                send!(sender, Action::OpenAddAccountDialog);
            })
        );

        let password_entry = self_.password_entry.get();
        action!(
            self,
            "unlock",
            clone!(@strong sender, @weak password_entry, @weak app => move |_, _| {
                let password = password_entry.get_text().unwrap();
                if Keyring::is_current_password(&password).unwrap() {
                    password_entry.set_text("");
                    app.set_locked(false);
                    send!(sender, Action::SetView(View::Accounts));
                }
            })
        );
    }

    fn setup_signals(&self, app: &Application, sender: Sender<Action>) {
        app.connect_local(
            "notify::locked",
            false,
            clone!(@weak app => move |_| {
                if app.locked(){
                    send!(sender, Action::SetView(View::Login));
                } else {
                    send!(sender, Action::SetView(View::Accounts));
                };
                None
            }),
        )
        .unwrap();
    }
}
