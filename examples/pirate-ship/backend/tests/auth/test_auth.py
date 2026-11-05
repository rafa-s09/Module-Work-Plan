import time

import pytest

from app import auth, crew
from app.errors import ShipError

PASSWORD = "seven-seas-rum"


def refused(code, call, *args, **kwargs):
    with pytest.raises(ShipError) as error:
        call(*args, **kwargs)
    assert error.value.code == code


@pytest.fixture
def jack(conn, ship):
    auth.create_account(conn, ship["sailor"], "jack", PASSWORD)
    return ship["sailor"]


def test_account_rules(conn, ship, jack):
    """TEST-0016: usernames follow the rules, are unique ignoring case; one account per member."""
    refused("invalid_input", auth.create_account, conn, ship["captain"], "ab", PASSWORD)
    refused("invalid_input", auth.create_account, conn, ship["captain"], "no spaces", PASSWORD)
    refused("username_taken", auth.create_account, conn, ship["captain"], "JACK", PASSWORD)
    refused("already_has_account", auth.create_account, conn, jack, "jack2", PASSWORD)


def test_passwords_are_hashed(conn, ship, jack):
    """TEST-0017: short passwords are refused; hashes are never the password and salts differ."""
    refused("weak_password", auth.create_account, conn, ship["captain"], "anne", "short")
    auth.create_account(conn, ship["captain"], "anne", PASSWORD)
    rows = conn.execute("SELECT * FROM accounts").fetchall()
    assert all(PASSWORD.encode() not in r["password_hash"] for r in rows)
    assert all(len(r["password_salt"]) == 16 for r in rows)
    assert rows[0]["password_salt"] != rows[1]["password_salt"]
    assert rows[0]["password_hash"] != rows[1]["password_hash"]


def test_login_stores_only_token_hash(conn, jack):
    """TEST-0018: login returns a token and stores only its SHA-256."""
    token = auth.login(conn, "Jack", PASSWORD)
    assert auth.current_member(conn, token)["id"] == jack
    stored = conn.execute("SELECT token_sha256 FROM sessions").fetchone()["token_sha256"]
    assert stored != token and len(stored) == 64


def test_same_answer_for_bad_name_and_password(conn, jack, monkeypatch):
    """TEST-0019: wrong name and wrong password answer the same, and both spend the hash."""
    calls = []
    real = auth._hash
    monkeypatch.setattr(auth, "_hash", lambda p, s: calls.append(p) or real(p, s))
    refused("bad_credentials", auth.login, conn, "jack", "wrong-password")
    refused("bad_credentials", auth.login, conn, "nobody", "wrong-password")
    assert len(calls) == 2


@pytest.mark.parametrize("username", ["jack", "nobody"])
def test_lock_after_five_failures(conn, jack, username):
    """TEST-0020: 5 failures lock a username, real or not, for 15 minutes; the lock expires."""
    for _ in range(5):
        refused("bad_credentials", auth.login, conn, username, "wrong-password")
    refused("locked", auth.login, conn, username, PASSWORD)
    conn.execute("UPDATE login_failures SET locked_until = '2000-01-01T00:00:00Z'")
    if username == "jack":
        assert auth.login(conn, username, PASSWORD)
        assert conn.execute("SELECT COUNT(*) AS n FROM login_failures").fetchone()["n"] == 0


def test_success_resets_the_count(conn, jack):
    """TEST-0021: a successful login resets the count of failures."""
    for _ in range(4):
        refused("bad_credentials", auth.login, conn, "jack", "wrong-password")
    auth.login(conn, "jack", PASSWORD)
    refused("bad_credentials", auth.login, conn, "jack", "wrong-password")
    assert auth.login(conn, "jack", PASSWORD)


def test_sessions_end(conn, jack):
    """TEST-0022: a session lasts 8 hours and is not extended; logout ends it at once."""
    token = auth.login(conn, "jack", PASSWORD)
    created, expires = conn.execute("SELECT created_at, expires_at FROM sessions").fetchone()
    assert int(expires[11:13]) == (int(created[11:13]) + 8) % 24
    auth.current_member(conn, token)
    assert conn.execute("SELECT expires_at FROM sessions").fetchone()[0] == expires
    auth.logout(conn, token)
    refused("not_logged_in", auth.current_member, conn, token)
    token = auth.login(conn, "jack", PASSWORD)
    conn.execute("UPDATE sessions SET expires_at = '2000-01-01T00:00:00Z'")
    refused("not_logged_in", auth.current_member, conn, token)


def test_first_captain_only_once(conn):
    """TEST-0023: the first captain is created once; with any account it refuses."""
    captain = auth.create_first_captain(conn, "Anne", "anne", PASSWORD)
    assert crew.get(conn, captain)["rank"] == "captain"
    refused("accounts_exist", auth.create_first_captain, conn, "Bart", "bart", PASSWORD)


def test_leaving_the_ship_ends_sessions(conn, ship, jack):
    """TEST-0024: leaving `aboard` ends sessions at once, for good; only `aboard` can log in."""
    token = auth.login(conn, "jack", PASSWORD)
    auth.change_status(conn, jack, "ashore", ship["captain"])
    refused("not_logged_in", auth.current_member, conn, token)
    refused("bad_credentials", auth.login, conn, "jack", PASSWORD)
    auth.change_status(conn, jack, "aboard", ship["captain"])
    refused("not_logged_in", auth.current_member, conn, token)
    assert auth.login(conn, "jack", PASSWORD)


def test_orders_by_rank(conn, ship):
    """TEST-0025: each order is allowed only to the ranks of the table."""
    assert auth.allowed_orders("captain") == ["manage_accounts", "manage_crew", "move_ship"]
    assert auth.allowed_orders("quartermaster") == ["change_hold", "manage_loot"]
    assert auth.allowed_orders("boatswain") == ["change_hold"]
    assert auth.allowed_orders("sailor") == []
    refused("not_allowed", auth.require, crew.get(conn, ship["sailor"]), "move_ship")
    auth.require(crew.get(conn, ship["captain"]), "move_ship")


def test_change_own_password(conn, jack):
    """TEST-0026: changing a password needs the current one and ends the other sessions."""
    other = auth.login(conn, "jack", PASSWORD)
    this = auth.login(conn, "jack", PASSWORD)
    refused("wrong_password", auth.change_own_password, conn, jack, "not-the-one", "brand-new-pass", this)
    refused("weak_password", auth.change_own_password, conn, jack, PASSWORD, "short", this)
    auth.change_own_password(conn, jack, PASSWORD, "brand-new-pass", this)
    assert auth.current_member(conn, this)["id"] == jack
    refused("not_logged_in", auth.current_member, conn, other)
    assert auth.login(conn, "jack", "brand-new-pass")


def test_captain_resets_password(conn, jack):
    """TEST-0027: a reset replaces the password and ends every session of the member."""
    token = auth.login(conn, "jack", PASSWORD)
    auth.reset_password(conn, jack, "brand-new-pass")
    refused("not_logged_in", auth.current_member, conn, token)
    assert auth.login(conn, "jack", "brand-new-pass")


def test_password_check_time(conn, jack):
    """TEST-0028: checking a password takes under 500 ms, right or wrong."""
    for password in (PASSWORD, "wrong-password"):
        start = time.perf_counter()
        try:
            auth.login(conn, "jack", password)
        except ShipError:
            pass
        assert time.perf_counter() - start < 0.5
