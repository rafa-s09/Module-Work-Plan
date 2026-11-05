import re
import sqlite3
import time

import pytest

from app import db


def test_migrations_apply_in_order(conn):
    """TEST-0001: every migration is applied once and recorded with its name and time."""
    rows = conn.execute("SELECT * FROM schema_version ORDER BY name").fetchall()
    assert [r["name"] for r in rows] == sorted(f.name for f in db.MIGRATIONS.glob("*.sql"))
    db._migrate(conn, db.MIGRATIONS)
    assert len(conn.execute("SELECT * FROM schema_version").fetchall()) == len(rows)


def test_foreign_keys_are_on(conn):
    """TEST-0002: a connection enforces foreign keys."""
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute("INSERT INTO accounts VALUES (999, 'ghost', x'00', x'00')")


def test_broken_migration_leaves_nothing(tmp_path):
    """TEST-0003: a migration that fails is rolled back and not recorded."""
    (tmp_path / "0001_broken.sql").write_text("CREATE TABLE half (id INTEGER); THIS IS NOT SQL;")
    with pytest.raises(sqlite3.Error):
        db.connect(":memory:", migrations=tmp_path)
    (tmp_path / "0001_broken.sql").write_text("CREATE TABLE half (id INTEGER);")
    conn = db.connect(":memory:", migrations=tmp_path)
    assert conn.execute("SELECT name FROM schema_version").fetchone()["name"] == "0001_broken.sql"


def test_transaction_rolls_back(conn):
    """TEST-0004: an exception inside a transaction undoes it, including a joined one."""
    insert = "INSERT INTO crew_members (name, rank, signed_at) VALUES ('Bo', 'sailor', 'x')"
    with pytest.raises(ValueError), db.transaction(conn):
        with db.transaction(conn):
            conn.execute(insert)
        raise ValueError
    assert conn.execute("SELECT COUNT(*) AS n FROM crew_members").fetchone()["n"] == 0


def test_file_path_from_environment(tmp_path, monkeypatch):
    """TEST-0005: the file comes from LEDGER_DB and its folder is created, in WAL mode."""
    path = tmp_path / "deep" / "ledger.db"
    monkeypatch.setenv("LEDGER_DB", str(path))
    conn = db.connect()
    assert path.exists()
    assert conn.execute("PRAGMA journal_mode").fetchone()[0] == "wal"
    conn.close()


def test_migrations_are_fast():
    """TEST-0006: all migrations on an empty in-memory database take under 1 second."""
    start = time.perf_counter()
    db.connect(":memory:").close()
    assert time.perf_counter() - start < 1


def test_times_are_utc_text():
    """TEST-0007: times are ISO 8601 UTC with seconds and a Z."""
    assert re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", db.now())
