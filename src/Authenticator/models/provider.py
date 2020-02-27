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
from os import path
from gi.repository import Gtk

from Authenticator.models import Database


class Provider:

    instance: 'Provider' = None

    def __init__(self, *args):
        self._provider_id = args[0]
        self.name = args[1]
        self.website = args[2]
        self.doc_url = args[3]
        self.image = args[4]
        self.period = args[5]
        self.type = args[6]

    @staticmethod
    def create(name: str, website: str, doc_url: str, image: str) -> 'Provider':
        provider = Database.get_default().insert_provider(name, website, doc_url, image)
        return provider

    @staticmethod
    def get_by_id(id_) -> 'Provider':
        provider = Database.get_default().provider_by_id(id_)
        return provider

    @staticmethod
    def get_by_name(name) -> 'Provider':
        provider = Database.get_default().provider_by_name(name)
        return provider

    @staticmethod
    def all() -> ['Provider']:
        providers = Database.get_default().get_providers()
        return providers

    @property
    def id(self):
        return self._provider_id

    @property
    def image_path(self) -> str:
        from Authenticator.widgets import ProviderImage
        cached_icon = path.join(ProviderImage.CACHE_DIR, self.image)
        if path.exists(cached_icon):
            return cached_icon
        else:
            theme = Gtk.IconTheme.get_default()
            icon_info = theme.lookup_icon("image-missing", 48, 0)
            if icon_info:
                return icon_info.get_filename()
        return None

    def update(self, **provider_data):
        self.name = provider_data.get("name", self.name)
        self.image = provider_data.get("image", self.image)
        Database.get_default().update_provider(provider_data, self._provider_id)
