use crate::helpers::Keyring;
use gio::{ActionExt, ActionMapExt};
use gtk::prelude::*;
use std::rc::Rc;
pub struct PasswordPage {
    pub widget: gtk::Box,
    builder: gtk::Builder,
    actions: gio::SimpleActionGroup,
}

impl PasswordPage {
    pub fn new(actions: gio::SimpleActionGroup) -> Rc<Self> {
        let builder = gtk::Builder::from_resource(
            "/com/belmoussaoui/Authenticator/preferences_password_page.ui",
        );
        get_widget!(builder, gtk::Box, password_page);
        let page = Rc::new(Self {
            widget: password_page,
            builder,
            actions,
        });
        page.init(page.clone());
        page
    }

    fn init(&self, page: Rc<Self>) {
        get_widget!(self.builder, gtk::PasswordEntry, current_password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, password_entry);
        get_widget!(self.builder, gtk::PasswordEntry, confirm_password_entry);
        get_widget!(self.builder, libhandy::ActionRow, current_password_row);

        let has_set_password = Keyring::has_set_password().unwrap_or_else(|_| false);

        let validate = clone!(@strong self.builder as builder,
            @weak current_password_entry,
            @weak password_entry,
            @weak self.actions as actions,
            @weak confirm_password_entry => move |_: &gtk::PasswordEntry| {

            let current_password = current_password_entry.get_text().unwrap();
            let password = password_entry.get_text().unwrap();
            let password_repeat = confirm_password_entry.get_text().unwrap();

            let is_valid = if has_set_password {
                password_repeat == password && current_password != password
                && password != ""
            } else {
                password_repeat == password && password != ""
            };

            get_action!(actions, @save_password).set_enabled(is_valid);
        });

        password_entry.connect_changed(validate.clone());
        confirm_password_entry.connect_changed(validate.clone());

        if !has_set_password {
            current_password_row.hide();
        } else {
            current_password_entry.connect_changed(validate.clone());
        }

        action!(
            self.actions,
            "save_password",
            clone!(@strong page => move |_, _| {
                page.save();
            })
        );
        get_action!(self.actions, @save_password).set_enabled(false);
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
            get_action!(self.actions, @close_page).activate(None);
        }
    }
}
