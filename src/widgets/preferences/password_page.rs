use crate::helpers::Keyring;
use gio::{ActionExt, ActionMapExt};
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub struct PasswordPage {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    actions: gio::SimpleActionGroup,
    has_set_password: Cell<bool>,
}

impl PasswordPage {
    pub fn new(actions: gio::SimpleActionGroup) -> Rc<Self> {
        let builder = gtk::Builder::from_resource(
            "/com/belmoussaoui/Authenticator/preferences_password_page.ui",
        );
        get_widget!(builder, gtk::Box, password_page);

        let has_set_password = Keyring::has_set_password().unwrap_or_else(|_| false);

        let page = Rc::new(Self {
            widget: password_page,
            builder,
            actions,
            has_set_password: Cell::new(has_set_password),
        });
        page.init(page.clone());
        page
    }

    fn validate(&self) {
        get_widget!(self.builder, gtk::PasswordEntry, current_password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, confirm_password_entry);

        let current_password = current_password_entry.get_text().unwrap();
        let password = password_entry.get_text().unwrap();
        let password_repeat = confirm_password_entry.get_text().unwrap();

        let is_valid = if self.has_set_password.get() {
            password_repeat == password && current_password != password && password != ""
        } else {
            password_repeat == password && password != ""
        };

        get_action!(self.actions, @save_password).set_enabled(is_valid);
    }

    fn init(&self, page: Rc<Self>) {
        get_widget!(self.builder, gtk::PasswordEntry, current_password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, confirm_password_entry);
        get_widget!(self.builder, libhandy::ActionRow, current_password_row);

        password_entry.connect_changed(clone!(@strong page => move |_| page.validate()));
        confirm_password_entry.connect_changed(clone!(@strong page => move |_| page.validate()));

        if !self.has_set_password.get() {
            current_password_row.hide();
        } else {
            current_password_entry
                .connect_changed(clone!(@strong page => move |_| page.validate()));
        }

        action!(
            self.actions,
            "save_password",
            clone!(@strong page => move |_, _| {
                page.save();
            })
        );

        action!(
            self.actions,
            "reset_password",
            clone!(@strong page => move |_,_| {
                page.reset();
            })
        );

        get_action!(self.actions, @save_password).set_enabled(false);
        get_action!(self.actions, @reset_password).set_enabled(self.has_set_password.get());
    }

    fn reset(&self) {
        if Keyring::reset_password().is_ok() {
            get_action!(self.actions, @close_page).activate(None);
            get_action!(self.actions, @save_password).set_enabled(false);
            get_action!(self.actions, @reset_password).set_enabled(false);
            get_widget!(self.builder, libhandy::ActionRow, @current_password_row).hide();
            self.has_set_password.set(false);
        }
    }

    fn save(&self) {
        get_widget!(self.builder, gtk::PasswordEntry, current_password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, confirm_password_entry);

        let current_password = current_password_entry.get_text().unwrap();
        let password = password_entry.get_text().unwrap();

        if Keyring::has_set_password().unwrap_or(false) {
            if !Keyring::is_current_password(&current_password).unwrap_or(false) {
                return;
            }
        }

        if Keyring::set_password(&password).is_ok() {
            get_widget!(self.builder, libhandy::ActionRow, @current_password_row).show();
            current_password_entry.set_text("");
            password_entry.set_text("");
            confirm_password_entry.set_text("");
            get_action!(self.actions, @save_password).set_enabled(false);
            get_action!(self.actions, @reset_password).set_enabled(true);
            get_action!(self.actions, @close_page).activate(None);
            self.has_set_password.set(true);
        }
    }
}
