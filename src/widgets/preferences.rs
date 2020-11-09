use crate::config;
use gio::SettingsExt;
use gtk::prelude::*;

pub struct PreferencesWindow {
    pub widget: libhandy::PreferencesWindow,
    builder: gtk::Builder,
    settings: gio::Settings,
}

impl PreferencesWindow {
    pub fn new() -> Self {
        let builder = gtk::Builder::from_resource("/com/belmoussaoui/Authenticator/preferences.ui");
        get_widget!(builder, libhandy::PreferencesWindow, preferences_window);
        let settings = gio::Settings::new(config::APP_ID);

        let preferences = Self {
            widget: preferences_window,
            builder,
            settings,
        };
        preferences.init();
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
}
