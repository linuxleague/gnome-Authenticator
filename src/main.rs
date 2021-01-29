#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
use gtk::{gio, glib};

use gettextrs::*;
mod application;
mod backup;
mod config;
mod models;
mod schema;
mod widgets;
mod window_state;

use application::Application;

fn main() {
    pretty_env_logger::init();
    gtk::init().expect("failed to init gtk4 ");
    gst::init().expect("failed to init gstreamer");

    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR);
    textdomain(config::GETTEXT_PACKAGE);

    let res = gio::Resource::load(config::PKGDATADIR.to_owned() + "/authenticator.gresource")
        .expect("Could not load resources");
    gio::resources_register(&res);

    glib::set_application_name(&gettext("Authenticator"));

    Application::run();
}
