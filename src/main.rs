extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
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

extern crate gtk;
use gettextrs::*;
use libhandy::ColumnExt;
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
    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);

    static_resources::init().expect("Failed to initialize the resource file.");

    let column = libhandy::Column::new();
    column.set_maximum_width(800);
    column.set_linear_growth_width(600);

    let app = Application::new();
    app.run(app.clone());
}
