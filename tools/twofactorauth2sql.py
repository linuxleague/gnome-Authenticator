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
    if set(["tfa", "software"]).issubset(provider.keys()):
        return provider["tfa"] and provider["software"]
    else:
        return False


output = {}

html_parser = HTMLParser()

down_query = ""
up_query = ""

for db_file in glob(DATA_DIR + "/*.yml"):
    with open(db_file, 'r', encoding='utf8') as file_data:
        try:
            providers = yaml.load(file_data)["websites"]
            for provider in providers:
                if is_valid(provider):
                	up_query += "INSERT INTO `providers` (`name`, `website`, `help_url`, `image_uri`) VALUES (`{}`, `{}`, `{}`, `{}`);\n".format(
                				html_parser.unescape(provider.get("name")), provider.get("url"), provider.get("doc", ""), '')
                	down_query += "DELETE FROM `providers` WHERE `name`=`{}`\n".format(html_parser.unescape(provider.get("name")));
        except (yaml.YAMLError, TypeError, KeyError) as error:
            pass

with open('./up.sql', 'w') as fo:
	fo.write(up_query)


with open('./down.sql', 'w') as fo:
	fo.write(down_query)

rmtree(TMP_FOLDER)

