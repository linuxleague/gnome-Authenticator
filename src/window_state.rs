use crate::widgets::Window;
use anyhow::Result;
use gtk::prelude::*;

pub fn load(window: &Window, settings: &gtk::gio::Settings) {
    let width = settings.int("window-width");
    let height = settings.int("window-height");

    if width > -1 && height > -1 {
        window.set_default_size(width, height);
    }

    let is_maximized = settings.boolean("is-maximized");
    if is_maximized {
        window.maximize();
    }
}

pub fn save(window: &Window, settings: &gtk::gio::Settings) -> Result<()> {
    let size = window.default_size();
    settings.set_int("window-width", size.0)?;
    settings.set_int("window-height", size.1)?;

    settings.set_boolean("is-maximized", window.is_maximized())?;
    Ok(())
}
