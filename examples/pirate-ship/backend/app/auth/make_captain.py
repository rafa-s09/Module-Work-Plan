"""Signs the first captain: python -m app.auth.make_captain (auth#FR-07)."""

import getpass

from app import auth
from app.db import connect
from app.errors import ShipError

if __name__ == "__main__":
    name = input("Captain's name: ")
    username = input("Username: ")
    password = getpass.getpass("Password (10 characters or more): ")
    try:
        auth.create_first_captain(connect(), name, username, password)
    except ShipError as error:
        raise SystemExit(error.message) from None
    print(f"Welcome aboard, Captain {name.strip()}!")
