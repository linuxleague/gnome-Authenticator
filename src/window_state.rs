use gio::prelude::SettingsExt;
use gtk::prelude::GtkWindowExt;

pub fn load(window: &libhandy::ApplicationWindow, settings: &gio::Settings) {
    let width = settings.get_int("window-width");
    let height = settings.get_int("window-height");

    if width > -1 && height > -1 {
        window.resize(360, 600);
    }

    let is_maximized = settings.get_boolean("is-maximized");
    if is_maximized {
        window.maximize();
    }
}

pub fn save(window: &libhandy::ApplicationWindow, settings: &gio::Settings) {
    let size = window.get_size();
    settings.set_int("window-width", size.0);
    settings.set_int("window-height", size.1);

    settings.set_boolean("is-maximized", window.is_maximized());
}
