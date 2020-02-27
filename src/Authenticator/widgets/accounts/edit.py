"""
 Copyright © 2017 Bilal Elmoussaoui <bil.elmoussaoui@gmail.com>

 This file is part of Authenticator.

 Authenticator is free software: you can redistribute it and/or
 modify it under the terms of the GNU General Public License as published
 by the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 Authenticator is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with Authenticator. If not, see <http://www.gnu.org/licenses/>.
"""
from gettext import gettext as _
from gi.repository import Gtk, GObject, Handy


class EditAccountWindow(Handy.Dialog):
    __gtype_name__ = 'EditAccountWindow'
    # Signals
    __gsignals__ = {
        'updated': (
            GObject.SignalFlags.RUN_LAST,
            None,
            (str, GObject.TYPE_PYOBJECT, )
        ),
    }

    def __init__(self, account):
        super(EditAccountWindow, self).__init__(use_header_bar=True, default_width=760, default_height=500)
        self._account = account
        self._save_button = Gtk.Button.new_with_label(_("Save"))
        self.__init_widgets()

    def __init_widgets(self):
        from .add import AccountConfig
        self._save_button.get_style_context().add_class("suggested-action")
        self._save_button.set_sensitive(True)
        self._save_button.set_can_default(True)

        self.get_header_bar().pack_end(self._save_button)

        self.account_config = AccountConfig(edit=True, account=self._account)
        self.account_config.connect("changed", self._on_account_config_changed)

        self.get_content_area().pack_start(self.account_config, True, True, 0)
        self._save_button.connect("clicked", self._on_save)

    def _on_account_config_changed(self, _, state: bool):
        """
        Set the sensitivity of the AddButton
            depends on the AccountConfig.

        :param state: the state of the save button
        :type state: bool
        """
        if state:
            self._save_button.grab_default()
        self._save_button.set_sensitive(state)

    def _on_save(self, *_):
        """
            Save Button clicked signal handler.
        """
        from .list import AccountsWidget
        ac_widget = AccountsWidget.get_default()

        new_account = self.account_config.account
        username = new_account["username"]
        provider = new_account["provider"]
        old_provider = self._account.provider
        # Update the AccountRow widget
        self.emit("updated", username, provider)
        # Update the providers list
        if provider.id != old_provider.id:
            ac_widget.update_provider(self._account, provider)

        ac_widget.update_provider_image(provider)
        self.destroy()
