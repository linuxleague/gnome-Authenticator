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

from gi import require_version

require_version("Gtk", "3.0")
from gi.repository import Gtk, GObject

from .edit import EditAccountWindow

@Gtk.Template(resource_path='/com/github/bilelmoussaoui/Authenticator/account_row.ui')
class AccountRow(Gtk.ListBoxRow, GObject.GObject):
    """
        AccountRow Widget.

        It's a subclass of Gtk.ListBoxRow
        Added as a child to a AccountsList

        @signals: None
        @properties: account
    """
    __gsignals__ = {
        'on_selected': (GObject.SignalFlags.RUN_LAST, None, ()),
    }

    __gtype_name__ = 'AccountRow'

    account_name_label = Gtk.Template.Child()
    pin_label = Gtk.Template.Child()

    def __init__(self, account):
        """
        :param account: Account
        """
        super(AccountRow, self).__init__()
        self.init_template('AccountRow')
        self._account = account

        self._account.connect("otp_updated", self._on_pin_updated)
        self.__init_widgets()

    @property
    def account(self):
        """
            The Account model assigned to this AccountRow

            :return: Account Object
        """
        return self._account


    def __init_widgets(self):
        # Set up account name text label
        self.account_name_label.set_text(self.account.username)
        self.account_name_label.set_tooltip_text(self.account.username)

        # Set up account pin text label
        pin = self.account.otp.pin
        if pin:
            self.pin_label.set_text(pin)
        else:
            self.pin_label.set_text("??????")
            self.pin_label.set_tooltip_text(_("Couldn't generate the secret code"))

    @Gtk.Template.Callback('copy_btn_clicked')
    def _on_copy(self, *_):
        """
            Copy button clicked signal handler.
            Copies the OTP pin to the clipboard
        """
        self._account.copy_pin()

    @Gtk.Template.Callback('edit_btn_clicked')
    def _on_edit(self, *_):
        """
            Edit Button clicked signal handler.
            Opens a new Window to edit the current account.
        """
        from ..window import Window
        edit_window = EditAccountWindow(self._account)
        edit_window.set_transient_for(Window.get_default())
        edit_window.connect("updated", self._on_update)
        edit_window.show_all()
        edit_window.present()

    def _on_update(self, _, username, provider):
        """
            On account update signal handler.
            Updates the account username and provider

            :param username: the new account's username
            :type username: str

            :param provider: the new account's provider
            :type provider: str
        """
        self.username_lbl.set_text(username)
        self.account.update(username, provider)

    def _on_pin_updated(self, _, pin):
        """
            Updates the pin label each time a new OTP is generated.
            otp_updated signal handler.

            :param pin: the new OTP
            :type pin: str
        """
        if pin:
            self.pin_label.set_text(pin)
