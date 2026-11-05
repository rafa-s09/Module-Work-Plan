"""The plunder: whole reales, the wounded paid first, then the split by shares."""

from app import voyage
from app.db import now, transaction
from app.errors import ShipError

REALES_PER_DOUBLOON = 32
REALES_PER_PIECE_OF_EIGHT = 8
INJURY_PIECES_OF_EIGHT = {"right_arm": 600, "left_arm": 500, "right_leg": 500, "left_leg": 400, "eye": 100, "finger": 100}


def format_reales(amount: int) -> str:
    doubloons, reales = divmod(amount, REALES_PER_DOUBLOON)
    return f"{doubloons} doubloon{'' if doubloons == 1 else 's'}, {reales} real{'' if reales == 1 else 'es'}"


def record_plunder(conn, voyage_id: int, reales: int, description: str, by: int) -> None:
    description = description.strip()
    if reales <= 0 or not 1 <= len(description) <= 120:
        raise ShipError("invalid_input")
    with transaction(conn):
        if voyage.get(conn, voyage_id)["state"] != "plundering":
            raise ShipError("not_plundering")
        conn.execute(
            "INSERT INTO plunder (voyage_id, amount_reales, description, by_member_id, at) VALUES (?, ?, ?, ?, ?)",
            (voyage_id, reales, description, by, now()),
        )


def record_compensation(conn, voyage_id: int, member_id: int, injury: str, by: int) -> None:
    if injury not in INJURY_PIECES_OF_EIGHT:
        raise ShipError("invalid_input")
    with transaction(conn):
        voyage.get(conn, voyage_id)
        if get_split(conn, voyage_id):
            raise ShipError("already_split")
        if member_id not in {r["crew_member_id"] for r in voyage.roster(conn, voyage_id)}:
            raise ShipError("not_on_roster")
        conn.execute(
            "INSERT INTO compensations (voyage_id, crew_member_id, injury, amount_reales, by_member_id, at) VALUES (?, ?, ?, ?, ?, ?)",
            (voyage_id, member_id, injury, INJURY_PIECES_OF_EIGHT[injury] * REALES_PER_PIECE_OF_EIGHT, by, now()),
        )


def plunder_entries(conn, voyage_id: int) -> list:
    return conn.execute("SELECT * FROM plunder WHERE voyage_id = ? ORDER BY id", (voyage_id,)).fetchall()


def compensation_entries(conn, voyage_id: int) -> list:
    return conn.execute(
        """SELECT x.*, c.name FROM compensations x JOIN crew_members c ON c.id = x.crew_member_id
           WHERE x.voyage_id = ? ORDER BY x.id""",
        (voyage_id,),
    ).fetchall()


def split(conn, voyage_id: int, by: int) -> None:
    with transaction(conn):
        if voyage.get(conn, voyage_id)["state"] not in ("returning", "docked"):
            raise ShipError("bad_split_time")
        if get_split(conn, voyage_id):
            raise ShipError("already_split")
        total = sum(p["amount_reales"] for p in plunder_entries(conn, voyage_id))
        if total == 0:
            raise ShipError("no_plunder")
        owed: dict[int, int] = {}
        for c in compensation_entries(conn, voyage_id):
            owed[c["crew_member_id"]] = owed.get(c["crew_member_id"], 0) + c["amount_reales"]
        compensation = sum(owed.values())
        if compensation > total:
            raise ShipError("plunder_too_small")

        rest = total - compensation
        roster = voyage.roster(conn, voyage_id)
        all_quarters = sum(r["quarter_shares"] for r in roster)
        lines = []
        for r in roster:
            share = rest * r["quarter_shares"] // all_quarters
            comp = owed.get(r["crew_member_id"], 0)
            paid_to = "chest" if r["status"] in ("dead", "marooned") else "member"
            lines.append((r["crew_member_id"], r["quarter_shares"], comp, share, comp + share, paid_to))
        leftover = rest - sum(line[3] for line in lines)
        if leftover < 0 or sum(line[4] for line in lines) + leftover != total:
            raise ShipError("split_broken")

        at = now()
        split_id = conn.execute(
            """INSERT INTO splits (voyage_id, plunder_reales, compensation_reales, leftover_reales, by_member_id, at)
               VALUES (?, ?, ?, ?, ?, ?)""",
            (voyage_id, total, compensation, leftover, by, at),
        ).lastrowid
        conn.executemany(
            f"""INSERT INTO split_lines
                (split_id, crew_member_id, quarter_shares, compensation_reales, share_reales, total_reales, paid_to)
                VALUES ({split_id}, ?, ?, ?, ?, ?, ?)""",
            lines,
        )
        to_chest = leftover + sum(line[4] for line in lines if line[5] == "chest")
        if to_chest:
            conn.execute("INSERT INTO chest (split_id, amount_reales, at) VALUES (?, ?, ?)", (split_id, to_chest, at))


def get_split(conn, voyage_id: int):
    return conn.execute("SELECT * FROM splits WHERE voyage_id = ?", (voyage_id,)).fetchone()


def split_lines(conn, split_id: int) -> list:
    return conn.execute(
        """SELECT l.*, c.name FROM split_lines l JOIN crew_members c ON c.id = l.crew_member_id
           WHERE l.split_id = ? ORDER BY l.crew_member_id""",
        (split_id,),
    ).fetchall()


def chest_balance(conn) -> int:
    return conn.execute("SELECT COALESCE(SUM(amount_reales), 0) AS total FROM chest").fetchone()["total"]
