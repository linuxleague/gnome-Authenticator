#!/usr/bin/env python3
"""
YAML database to JSON converter.
"""
import json
import tempfile
from glob import glob
from os import path, remove
from shutil import rmtree
from subprocess import call
from urllib.parse import urlparse

GIT_CLONE_URI = "https://github.com/2factorauth/twofactorauth"
TMP_FOLDER = path.join(tempfile.gettempdir(), "Authenticator")
DATA_DIR = path.join(TMP_FOLDER, "entries")

LAST_DATA = path.realpath(
    path.join(
        path.dirname(path.realpath(__file__)),
        "../migrations/2019-09-02-132153_fill_providers/data.json",
    )
)

with open(LAST_DATA, "r") as f:
    current_data = json.load(f)


print("Cloning the repository...")
if path.exists(TMP_FOLDER):
    rmtree(TMP_FOLDER)
call(["git", "clone", "--depth=1", GIT_CLONE_URI, TMP_FOLDER])


def is_valid(provider: dict) -> bool:
    return "totp" in provider.get("tfa", [])


def compare_url(website1: str, website2: str) -> bool:
    w1 = urlparse(website1)
    w2 = urlparse(website2)
    return w1.netloc.lstrip("www.").rstrip("/") == w2.netloc.lstrip("www.").rstrip("/")


def find_entry(current_data: dict, name: str, website: str) -> dict:
    for entry in current_data:
        if entry["name"] == name or (
            website and entry["website"] and compare_url(website, entry["website"])
        ):
            return entry


output = {}

down_query = ""
up_query = ""

for db_file in glob(DATA_DIR + "/**/*.json"):
    with open(db_file, "r", encoding="utf8") as file_data:
        try:
            data = json.load(file_data)
            provider = list(data.values())[0]
            name = list(data.keys())[0].replace("&amp;", "&")
            if is_valid(provider):
                website = provider.get("domain", "")
                if not website.startswith("http"):
                    website = f"https://www.{website}/"
                help_url = provider.get("documentation", "")
                old_entry = find_entry(current_data, name, website)

                if old_entry is not None:
                    update_entries = []
                    downgrade_entries = []
                    if not compare_url(website, old_entry["website"]):
                        update_entries.append(("website", website))
                        downgrade_entries.append(("website", old_entry["website"]))
                    if (
                        help_url
                        and old_entry["documentation"]
                        and not compare_url(help_url, old_entry["documentation"])
                    ):
                        update_entries.append(("help_url", help_url))
                        downgrade_entries.append(
                            ("help_url", old_entry["documentation"])
                        )

                    if name != old_entry["name"]:
                        up_condition = f'name="{old_entry["name"]}"'
                        down_condition = f'name="{name}"'
                        update_entries.append(("name", name))
                        downgrade_entries.append(("name", old_entry["name"]))
                    else:
                        up_condition = f'name="{name}"'
                        down_condition = f'name="{old_entry["name"]}"'

                    if len(update_entries) > 0:
                        up_columns = ""
                        i = 0
                        for (column, value) in update_entries:
                            up_columns += f'{column}="{value}"'
                            if i != len(update_entries) - 1:
                                up_columns += ", "
                            i += 1
                        down_columns = ""
                        i = 0
                        for (column, value) in downgrade_entries:
                            down_columns += f'{column}="{value}"'
                            if i != len(downgrade_entries) - 1:
                                down_columns += ", "
                            i += 1

                        up_query += f'UPDATE "providers" SET {up_columns} WHERE {up_condition};\n'
                        down_query += f'UPDATE "providers" SET {down_columns} WHERE {down_condition};\n'
                else:
                    up_query += f'INSERT INTO "providers" ("name", "website", "help_url") VALUES ("{name}", "{website}", "{help_url}");\n'
                    down_query += f'DELETE FROM "providers" WHERE "name"="{name}";\n'
        except (TypeError, KeyError) as error:
            print(error)

with open("./up.sql", "w") as fo:
    fo.write(up_query)


with open("./down.sql", "w") as fo:
    fo.write(down_query)

rmtree(TMP_FOLDER)
