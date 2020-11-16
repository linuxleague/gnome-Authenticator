use super::password_page::PasswordPage;
use super::provider_page::ProviderPage;
use super::providers_page::ProvidersPage;
use crate::config;
use crate::models::{Provider, ProvidersModel};
use gio::ActionMapExt;
use gio::SettingsExt;
use glib::{Receiver, Sender};
use gtk::prelude::*;
use libhandy::PreferencesWindowExt;
use std::cell::RefCell;
use std::rc::Rc;

pub enum PreferencesAction {
    EditProvider(Provider),
}

pub struct PreferencesWindow {
    pub widget: libhandy::PreferencesWindow,
    builder: gtk::Builder,
    settings: gio::Settings,
    providers_model: Rc<ProvidersModel>,
    password_page: Rc<PasswordPage>,
    providers_page: Rc<ProvidersPage>,
    provider_page: Rc<ProviderPage>,
    actions: gio::SimpleActionGroup,
    sender: Sender<PreferencesAction>,
    receiver: RefCell<Option<Receiver<PreferencesAction>>>,
}

impl PreferencesWindow {
    pub fn new(providers_model: Rc<ProvidersModel>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/preferences.ui");
        get_widget!(builder, libhandy::PreferencesWindow, preferences_window);
        let settings = gio::Settings::new(config::APP_ID);
        let actions = gio::SimpleActionGroup::new();
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        let preferences = Rc::new(Self {
            widget: preferences_window,
            builder,
            settings,
            providers_page: ProvidersPage::new(providers_model.clone(), sender.clone()),
            provider_page: ProviderPage::new(sender.clone()),
            password_page: PasswordPage::new(actions.clone()),

            providers_model,
            actions,
            sender,
            receiver,
        });
        preferences.init(preferences.clone());
        preferences.setup_actions();
        preferences
    }

    fn init(&self, preferences: Rc<Self>) {
        get_widget!(self.builder, gtk::Switch, dark_theme_switch);
        self.settings.bind(
            "dark-theme",
            &dark_theme_switch,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );

        get_widget!(self.builder, gtk::Switch, auto_lock_switch);
        self.settings.bind(
            "auto-lock",
            &auto_lock_switch,
            "active",
            gio::SettingsBindFlags::DEFAULT,
        );

        get_widget!(self.builder, gtk::SpinButton, lock_timeout_spin_btn);
        auto_lock_switch
            .bind_property("active", &lock_timeout_spin_btn, "sensitive")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .build();

        self.widget.add(&self.providers_page.widget);

        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(None, move |action| preferences.do_action(action));
    }

    fn do_action(&self, action: PreferencesAction) -> glib::Continue {
        match action {
            PreferencesAction::EditProvider(provider) => {
                self.provider_page.set_provider(provider);
                self.widget.present_subpage(&self.provider_page.widget)
            }
        }
        glib::Continue(true)
    }

    fn setup_actions(&self) {
        action!(
            self.actions,
            "show_password_page",
            clone!(@strong self.builder as builder,
                @strong self.password_page.widget as password_page,
                @strong self.widget as widget => move |_, _| {
                widget.present_subpage(&password_page);
            })
        );
        action!(
            self.actions,
            "close_page",
            clone!(@strong self.builder as builder,
                @strong self.widget as widget => move |_, _| {
                widget.close_subpage();
            })
        );
        self.widget
            .insert_action_group("preferences", Some(&self.actions));
    }
}
