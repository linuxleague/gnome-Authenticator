"""
Simple script to retrieve your accounts in case your system is broken:

Update the db_path variable to point to the database and
the output one to where you want the file to be saved

the resulted file can be re-imported by any application accepting FreeOTP like backups
which are a otp URI per line
"""

import sqlite3
from gi import require_version

require_version("Secret", "1")
from gi.repository import Secret


db_path = "/home/bilelmoussaoui/.var/app/com.belmoussaoui.Authenticator/data/authenticator/authenticator.db"
output = "/home/bilelmoussaoui/output.text"


db = sqlite3.connect(db_path)

accounts = db.execute(
    """
    SELECT * FROM accounts
    INNER JOIN providers ON accounts.provider_id = providers.id
    """
).fetchall()

service = Secret.Service.get_sync(Secret.ServiceFlags.NONE, None)
service.load_collections_sync(None)
collections = service.get_collections()
service.unlock_sync(collections, None)

default_collection = Secret.Collection.for_alias_sync(
    service, "default", Secret.CollectionFlags.NONE, None
)


def get_token(token_id):

    items = default_collection.search_sync(
        None,
        {
            "type": "token",
            "application": "com.belmoussaoui.Authenticator",
            "token_id": token_id,
        },
        Secret.SearchFlags.NONE,
        None,
    )
    if items:
        item = items[0]
        item.load_secret_sync(None)

        return item.get_secret().get_text()


accounts_otp_uri = []

for account in accounts:
    token_id = account[3]
    token = get_token(token_id)
    if not token:
        print(f"Couldn't find token for {account[1]}")
        continue
    method = account[-1]

    uri = f"otpauth://{method}/{account[1]}?secret={token}&issuer={account[6]}&algorithm={account[-2]}&digits={account[-5]}"

    if method == "totp":
        uri += f"&period={account[-4]}"
    else:
        uri += f"&counter={account[-3]}"
    uri += "\n"

    accounts_otp_uri.append(uri)

with open(output, "w") as f:
    f.writelines(accounts_otp_uri)
