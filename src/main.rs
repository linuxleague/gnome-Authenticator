#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use gettextrs::*;
mod application;
mod backup;
mod config;
mod helpers;
mod models;
mod schema;
mod static_resources;
mod widgets;
mod window_state;

use application::Application;
use config::{GETTEXT_PACKAGE, LOCALEDIR};

fn main() {
    pretty_env_logger::init();
    gtk::init().expect("failed to init gtk4 ");
    gst::init().expect("failed to init gstreamer");
    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);

    gtk::glib::set_application_name("Authenticator");
    gtk::glib::set_prgname(Some(config::APP_ID));

    static_resources::init().expect("Failed to initialize the resource file.");

    Application::run();
}
