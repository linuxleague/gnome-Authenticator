#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate glib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate gtk_macros;

use gettextrs::*;
mod application;
mod config;
mod models;
mod schema;
mod static_resources;
mod widgets;
mod window_state;

use application::Application;
use config::{GETTEXT_PACKAGE, LOCALEDIR};

fn main() {
    pretty_env_logger::init();

    gtk::init().expect("Unable to start GTK3");
    libhandy::functions::init();
    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);

    glib::set_application_name("Authenticator");
    glib::set_prgname(Some("authenticator"));

    static_resources::init().expect("Failed to initialize the resource file.");

    let app = Application::new();
    app.run(app.clone());
}
