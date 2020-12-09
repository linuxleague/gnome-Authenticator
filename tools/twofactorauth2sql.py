#!/usr/bin/env python3
"""
YAML database to JSON converter.
"""
import json
import tempfile
from collections import OrderedDict
from glob import glob
from os import path, remove
from shutil import rmtree
from subprocess import call
import sys
try:
    import yaml
except ImportError:
    sys.exit("Please install pyaml first")

try:
    from HTMLParser import HTMLParser
except ImportError:
    from html.parser import HTMLParser



GIT_CLONE_URI = "https://github.com/2factorauth/twofactorauth"
TMP_FOLDER = path.join(tempfile.gettempdir(), "Authenticator")
DATA_DIR = path.join(TMP_FOLDER, "_data")
OUTPUT_DIR = path.join(path.dirname(
    path.realpath(__file__)), "../data/data.json")

print("Cloning the repository...")
if path.exists(TMP_FOLDER):
    rmtree(TMP_FOLDER)
call(["git", "clone", "--depth=1", GIT_CLONE_URI, TMP_FOLDER])

if path.exists(OUTPUT_DIR):
    remove(OUTPUT_DIR)


def is_valid(provider):
    return "totp" in provider.get("tfa", [])


output = {}

html_parser = HTMLParser()

down_query = ""
up_query = ""

for db_file in glob(DATA_DIR + "/*.yml"):
    with open(db_file, 'r', encoding='utf8') as file_data:
        try:
            providers = yaml.load(file_data, Loader=yaml.SafeLoader)["websites"]
            for provider in providers:
                if is_valid(provider):
                    name = provider.get("name").replace("&amp;", "&")
                    website = provider.get("url", "")
                    help_url = provider.get("doc", "")
                    up_query += f'INSERT INTO "providers" ("name", "website", "help_url") VALUES ("{name}", "{website}", "{help_url}");\n'
                    down_query += f'DELETE FROM "providers" WHERE "name"="{name}";\n'
        except (yaml.YAMLError, TypeError, KeyError) as error:
            pass

with open('./up.sql', 'w') as fo:
	fo.write(up_query)


with open('./down.sql', 'w') as fo:
	fo.write(down_query)

rmtree(TMP_FOLDER)

