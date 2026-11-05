"""The strongroom: the SQLite connection, migrations and transactions."""

import os
import sqlite3
from contextlib import contextmanager
from datetime import UTC, datetime
from pathlib import Path

BACKEND = Path(__file__).resolve().parents[2]
MIGRATIONS = BACKEND / "migrations"


def to_text(moment: datetime) -> str:
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


def now() -> str:
    return to_text(datetime.now(UTC))


def connect(path: str | None = None, migrations: Path = MIGRATIONS) -> sqlite3.Connection:
    path = path or os.environ.get("LEDGER_DB") or str(BACKEND / "data" / "ledger.db")
    if path != ":memory:":
        Path(path).parent.mkdir(parents=True, exist_ok=True)
    # isolation_level=None: transactions are only the ones `transaction` opens.
    conn = sqlite3.connect(path, isolation_level=None, check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA journal_mode = WAL")
    _migrate(conn, migrations)
    return conn


def _migrate(conn: sqlite3.Connection, folder: Path) -> None:
    conn.execute("CREATE TABLE IF NOT EXISTS schema_version (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL)")
    done = {row["name"] for row in conn.execute("SELECT name FROM schema_version")}
    for file in sorted(folder.glob("*.sql")):
        if file.name in done:
            continue
        # executescript has no parameters; the name comes from our own migrations folder.
        script = f"""BEGIN;
{file.read_text(encoding="utf-8")}
INSERT INTO schema_version VALUES ('{file.name}', '{now()}');
COMMIT;"""
        try:
            conn.executescript(script)
        except sqlite3.Error:
            if conn.in_transaction:
                conn.rollback()
            raise


@contextmanager
def transaction(conn: sqlite3.Connection):
    """Commits on success, rolls back on any exception. Inside another one, it joins it."""
    if conn.in_transaction:
        yield
        return
    conn.execute("BEGIN IMMEDIATE")
    try:
        yield
    except BaseException:
        conn.rollback()
        raise
    conn.commit()
