use super::password_page::PasswordPage;
use crate::config;
use gio::ActionMapExt;
use gio::SettingsExt;
use gtk::prelude::*;
use libhandy::PreferencesWindowExt;
use std::rc::Rc;

pub struct PreferencesWindow {
    pub widget: libhandy::PreferencesWindow,
    builder: gtk::Builder,
    settings: gio::Settings,
    password_page: Rc<PasswordPage>,
    actions: gio::SimpleActionGroup,
}

impl PreferencesWindow {
    pub fn new() -> Self {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/preferences.ui");
        get_widget!(builder, libhandy::PreferencesWindow, preferences_window);
        let settings = gio::Settings::new(config::APP_ID);
        let actions = gio::SimpleActionGroup::new();
        let preferences = Self {
            widget: preferences_window,
            builder,
            settings,
            password_page: PasswordPage::new(actions.clone()),
            actions,
        };
        preferences.init();
        preferences.setup_actions();
        preferences
    }

    fn init(&self) {
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
