"""The course: a voyage's states, and the roster fixed when she sets sail."""

from app import crew, hold
from app.db import now, transaction
from app.errors import ShipError

COURSES = {
    "docked": {"sailing"},
    "sailing": {"plundering", "sunk"},
    "plundering": {"returning", "sailing", "sunk"},
    "returning": {"docked", "sunk"},
    "sunk": set(),
}
AT_SEA = ("sailing", "plundering", "returning")


def active(conn):
    return conn.execute(
        f"SELECT * FROM voyages WHERE state IN ({','.join('?' * len(AT_SEA))})", AT_SEA
    ).fetchone()


def get(conn, voyage_id: int):
    row = conn.execute("SELECT * FROM voyages WHERE id = ?", (voyage_id,)).fetchone()
    if row is None:
        raise ShipError("not_found")
    return row


def everything(conn) -> list:
    return conn.execute("SELECT * FROM voyages ORDER BY id DESC").fetchall()


def create(conn, name: str, destination: str, by: int) -> int:
    name, destination = name.strip(), destination.strip()
    if not 1 <= len(name) <= 80 or not 1 <= len(destination) <= 80:
        raise ShipError("invalid_input")
    with transaction(conn):
        if active(conn):
            raise ShipError("ship_at_sea")
        voyage_id = conn.execute(
            "INSERT INTO voyages (name, destination) VALUES (?, ?)", (name, destination)
        ).lastrowid
        _record(conn, voyage_id, "docked", by)
        return voyage_id


def _record(conn, voyage_id: int, state: str, by: int) -> None:
    conn.execute(
        "INSERT INTO voyage_events (voyage_id, state, by_member_id, at) VALUES (?, ?, ?, ?)",
        (voyage_id, state, by, now()),
    )


def roster(conn, voyage_id: int) -> list:
    return conn.execute(
        """SELECT r.crew_member_id, r.quarter_shares, c.name, c.rank, c.status
           FROM voyage_roster r JOIN crew_members c ON c.id = r.crew_member_id
           WHERE r.voyage_id = ? ORDER BY r.crew_member_id""",
        (voyage_id,),
    ).fetchall()


def events(conn, voyage_id: int) -> list:
    return conn.execute(
        "SELECT state, at FROM voyage_events WHERE voyage_id = ? ORDER BY id", (voyage_id,)
    ).fetchall()


def _set_sail(conn, voyage_id: int) -> None:
    if active(conn):
        raise ShipError("ship_at_sea")
    if roster(conn, voyage_id):
        raise ShipError("bad_course")  # she came home; a new trip is a new voyage
    in_hold = hold.counts(conn)
    if in_hold["water"] < 1 or in_hold["rations"] < 1:
        raise ShipError("empty_hold")
    conn.executemany(
        "INSERT INTO voyage_roster (voyage_id, crew_member_id, quarter_shares) VALUES (?, ?, ?)",
        [(voyage_id, m["id"], crew.quarter_shares(m["rank"])) for m in crew.aboard(conn)],
    )


def change_state(conn, voyage_id: int, state: str, by: int) -> None:
    if state not in COURSES:
        raise ShipError("invalid_input")
    with transaction(conn):
        voyage = get(conn, voyage_id)
        if state not in COURSES[voyage["state"]]:
            raise ShipError("bad_course")
        if voyage["state"] == "docked":
            _set_sail(conn, voyage_id)
        conn.execute("UPDATE voyages SET state = ? WHERE id = ?", (state, voyage_id))
        _record(conn, voyage_id, state, by)
