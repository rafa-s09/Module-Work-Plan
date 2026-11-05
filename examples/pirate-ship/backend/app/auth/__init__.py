"""The gangplank: accounts, passwords, sessions and who may give which order."""

import hashlib
import hmac
import re
import secrets
from datetime import UTC, datetime, timedelta

from app import crew
from app.db import now, to_text, transaction
from app.errors import ShipError

SESSION_HOURS = 8
MAX_FAILURES = 5
LOCK_MINUTES = 15
USERNAME = re.compile(r"[A-Za-z0-9_-]{3,32}")
# Spent on unknown usernames, so they take as long as known ones (auth#FR-04).
DUMMY_SALT = b"\x00" * 16

ORDERS = {
    "manage_accounts": {"captain"},
    "manage_crew": {"captain"},
    "move_ship": {"captain"},
    "manage_loot": {"quartermaster"},
    "change_hold": {"quartermaster", "boatswain"},
}


def _hash(password: str, salt: bytes) -> bytes:
    return hashlib.scrypt(password.encode(), salt=salt, n=2**14, r=8, p=1)


def _sha(token: str) -> str:
    return hashlib.sha256(token.encode()).hexdigest()


def _in(**delta) -> str:
    return to_text(datetime.now(UTC) + timedelta(**delta))


def _check_strength(password: str) -> None:
    if len(password) < 10:
        raise ShipError("weak_password")


def _store_password(conn, member_id: int, password: str) -> None:
    salt = secrets.token_bytes(16)
    conn.execute(
        "UPDATE accounts SET password_hash = ?, password_salt = ? WHERE crew_member_id = ?",
        (_hash(password, salt), salt, member_id),
    )


def create_account(conn, member_id: int, username: str, password: str) -> None:
    if not USERNAME.fullmatch(username):
        raise ShipError("invalid_input")
    _check_strength(password)
    with transaction(conn):
        crew.get(conn, member_id)
        if conn.execute("SELECT 1 FROM accounts WHERE crew_member_id = ?", (member_id,)).fetchone():
            raise ShipError("already_has_account")
        if conn.execute("SELECT 1 FROM accounts WHERE username = ?", (username,)).fetchone():
            raise ShipError("username_taken")
        salt = secrets.token_bytes(16)
        conn.execute(
            "INSERT INTO accounts (crew_member_id, username, password_hash, password_salt) VALUES (?, ?, ?, ?)",
            (member_id, username, _hash(password, salt), salt),
        )


def create_first_captain(conn, name: str, username: str, password: str) -> int:
    """The only way the first account is born (auth#FR-07)."""
    with transaction(conn):
        if conn.execute("SELECT 1 FROM accounts").fetchone():
            raise ShipError("accounts_exist")
        member_id = crew.add_member(conn, name, "captain")
        create_account(conn, member_id, username, password)
        return member_id


def _fail_login(conn, key: str) -> None:
    with transaction(conn):
        row = conn.execute("SELECT count FROM login_failures WHERE username = ?", (key,)).fetchone()
        count = (row["count"] if row else 0) + 1
        locked_until = _in(minutes=LOCK_MINUTES) if count >= MAX_FAILURES else None
        conn.execute(
            "INSERT OR REPLACE INTO login_failures (username, count, locked_until) VALUES (?, ?, ?)",
            (key, 0 if locked_until else count, locked_until),
        )


def login(conn, username: str, password: str) -> str:
    key = username.lower()
    failure = conn.execute("SELECT locked_until FROM login_failures WHERE username = ?", (key,)).fetchone()
    if failure and failure["locked_until"] and failure["locked_until"] > now():
        raise ShipError("locked")

    account = conn.execute(
        """SELECT a.*, c.status FROM accounts a JOIN crew_members c ON c.id = a.crew_member_id
           WHERE a.username = ?""",
        (username,),
    ).fetchone()
    given = _hash(password, account["password_salt"] if account else DUMMY_SALT)
    if account is None or not hmac.compare_digest(given, account["password_hash"]) or account["status"] != "aboard":
        _fail_login(conn, key)
        raise ShipError("bad_credentials")

    token = secrets.token_urlsafe(32)
    with transaction(conn):
        conn.execute("DELETE FROM login_failures WHERE username = ?", (key,))
        conn.execute(
            "INSERT INTO sessions (token_sha256, crew_member_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
            (_sha(token), account["crew_member_id"], now(), _in(hours=SESSION_HOURS)),
        )
    return token


def logout(conn, token: str) -> None:
    conn.execute("UPDATE sessions SET ended_at = ? WHERE token_sha256 = ? AND ended_at IS NULL", (now(), _sha(token)))


def current_member(conn, token: str):
    # Looked up by hash: an attacker can't steer the comparison, so no constant-time compare is needed.
    session = conn.execute(
        "SELECT crew_member_id FROM sessions WHERE token_sha256 = ? AND ended_at IS NULL AND expires_at > ?",
        (_sha(token), now()),
    ).fetchone()
    if session is None:
        raise ShipError("not_logged_in")
    member = crew.get(conn, session["crew_member_id"])
    if member["status"] != "aboard":
        raise ShipError("not_logged_in")
    return member


def end_sessions(conn, member_id: int, keep_token: str | None = None) -> None:
    conn.execute(
        "UPDATE sessions SET ended_at = ? WHERE crew_member_id = ? AND ended_at IS NULL AND token_sha256 != ?",
        (now(), member_id, _sha(keep_token) if keep_token else ""),
    )


def change_status(conn, member_id: int, status: str, by: int) -> None:
    with transaction(conn):
        crew.change_status(conn, member_id, status, by)
        if status != "aboard":
            end_sessions(conn, member_id)


def change_own_password(conn, member_id: int, current: str, new: str, token: str) -> None:
    _check_strength(new)
    account = conn.execute("SELECT * FROM accounts WHERE crew_member_id = ?", (member_id,)).fetchone()
    if not hmac.compare_digest(_hash(current, account["password_salt"]), account["password_hash"]):
        raise ShipError("wrong_password")
    with transaction(conn):
        _store_password(conn, member_id, new)
        end_sessions(conn, member_id, keep_token=token)


def reset_password(conn, member_id: int, new: str) -> None:
    _check_strength(new)
    with transaction(conn):
        if not conn.execute("SELECT 1 FROM accounts WHERE crew_member_id = ?", (member_id,)).fetchone():
            raise ShipError("not_found")
        _store_password(conn, member_id, new)
        end_sessions(conn, member_id)


def require(member, order: str) -> None:
    if member["rank"] not in ORDERS[order]:
        raise ShipError("not_allowed")


def allowed_orders(rank: str) -> list[str]:
    return sorted(order for order, ranks in ORDERS.items() if rank in ranks)
