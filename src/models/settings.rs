use std::ops::Deref;

use gtk::{gio, glib, prelude::*};

use crate::config;

pub struct Settings(gio::Settings);

impl Settings {
    pub fn download_favicons(&self) -> bool {
        self.boolean("download-favicons")
    }

    pub fn connect_download_favicons_changed<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(bool) + 'static,
    {
        self.connect_changed(Some("download-favicons"), move |settings, _key| {
            callback(settings.boolean("download-favicons"))
        })
    }

    pub fn download_favicons_metered(&self) -> bool {
        self.boolean("download-favicons-metered")
    }

    pub fn connect_download_favicons_metered_changed<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(bool) + 'static,
    {
        self.connect_changed(Some("download-favicons-metered"), move |settings, _key| {
            callback(settings.boolean("download-favicons-metered"))
        })
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(gio::Settings::new(config::APP_ID))
    }
}

impl Deref for Settings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}
