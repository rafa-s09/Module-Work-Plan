"""The ship's articles: ranks, shares, and what became of every soul who signed."""

from app.db import now, transaction
from app.errors import ShipError

# Share values live here and nowhere else (crew#FR-02).
QUARTER_SHARES = {"captain": 8, "quartermaster": 6, "boatswain": 5, "gunner": 5, "sailor": 4, "cabin_boy": 2}
ONE_ABOARD = {"captain", "quartermaster"}
STATUS_CHANGES = {"aboard": {"ashore", "marooned", "dead"}, "ashore": {"aboard"}, "marooned": set(), "dead": set()}


def quarter_shares(rank: str) -> int:
    return QUARTER_SHARES[rank]


def shares_display(quarters: int) -> str:
    whole, part = divmod(quarters, 4)
    return f"{whole or ''}{['', '¼', '½', '¾'][part]}" or "0"


def get(conn, member_id: int):
    row = conn.execute("SELECT * FROM crew_members WHERE id = ?", (member_id,)).fetchone()
    if row is None:
        raise ShipError("not_found")
    return row


def everyone(conn) -> list:
    return conn.execute("SELECT * FROM crew_members ORDER BY id").fetchall()


def aboard(conn) -> list:
    return conn.execute("SELECT * FROM crew_members WHERE status = 'aboard' ORDER BY id").fetchall()


def _check_name_free(conn, name: str) -> None:
    if conn.execute("SELECT 1 FROM crew_members WHERE status = 'aboard' AND name = ? COLLATE NOCASE", (name,)).fetchone():
        raise ShipError("name_taken")


def _check_rank_free(conn, rank: str) -> None:
    if rank in ONE_ABOARD and conn.execute(
        "SELECT 1 FROM crew_members WHERE status = 'aboard' AND rank = ?", (rank,)
    ).fetchone():
        raise ShipError("rank_taken")


def add_member(conn, name: str, rank: str) -> int:
    name = name.strip()
    if not 1 <= len(name) <= 60 or rank not in QUARTER_SHARES:
        raise ShipError("invalid_input")
    with transaction(conn):
        _check_name_free(conn, name)
        _check_rank_free(conn, rank)
        return conn.execute(
            "INSERT INTO crew_members (name, rank, signed_at) VALUES (?, ?, ?)", (name, rank, now())
        ).lastrowid


def _record(conn, member_id: int, field: str, old: str, new: str, by: int) -> None:
    conn.execute(
        "INSERT INTO crew_changes (crew_member_id, field, old_value, new_value, by_member_id, at) VALUES (?, ?, ?, ?, ?, ?)",
        (member_id, field, old, new, by, now()),
    )


def _set_rank(conn, member_id: int, old: str, new: str, by: int) -> None:
    conn.execute("UPDATE crew_members SET rank = ? WHERE id = ?", (new, member_id))
    _record(conn, member_id, "rank", old, new, by)


def change_rank(conn, member_id: int, rank: str, by: int) -> None:
    if rank not in QUARTER_SHARES:
        raise ShipError("invalid_input")
    with transaction(conn):
        member = get(conn, member_id)
        if member["rank"] == rank:
            return
        if member["rank"] == "captain" and member["status"] == "aboard":
            raise ShipError("rank_taken")  # a captain only steps down when another is made
        if rank == "captain":
            if member["status"] != "aboard":
                raise ShipError("not_aboard")
            old_captain = conn.execute("SELECT id FROM crew_members WHERE status = 'aboard' AND rank = 'captain'").fetchone()
            if old_captain:
                _set_rank(conn, old_captain["id"], "captain", "sailor", by)
        elif member["status"] == "aboard":
            _check_rank_free(conn, rank)
        _set_rank(conn, member_id, member["rank"], rank, by)


def change_status(conn, member_id: int, status: str, by: int) -> None:
    """Sessions are ended by auth.change_status, the door the API uses (auth#FR-08)."""
    if status not in STATUS_CHANGES:
        raise ShipError("invalid_input")
    with transaction(conn):
        member = get(conn, member_id)
        if member["status"] == "aboard" and member["rank"] == "captain":
            raise ShipError("captain_required")
        if status not in STATUS_CHANGES[member["status"]]:
            raise ShipError("bad_status_change")
        if status == "aboard":
            _check_name_free(conn, member["name"])
            _check_rank_free(conn, member["rank"])
        conn.execute("UPDATE crew_members SET status = ? WHERE id = ?", (status, member_id))
        _record(conn, member_id, "status", member["status"], status, by)
