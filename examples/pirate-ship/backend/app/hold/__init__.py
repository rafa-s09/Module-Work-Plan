"""Below deck: what the ship carries, counted as a sum of signed changes."""

from app.db import now, transaction
from app.errors import ShipError

STONE_PER_UNIT = {"rum": 4, "powder": 3, "rations": 2, "water": 5}
CAPACITY_STONE = 2000


def counts(conn) -> dict[str, int]:
    totals = dict.fromkeys(STONE_PER_UNIT, 0)
    for row in conn.execute("SELECT item, SUM(amount) AS total FROM hold_changes GROUP BY item"):
        totals[row["item"]] = row["total"]
    return totals


def weight(conn) -> int:
    return sum(count * STONE_PER_UNIT[item] for item, count in counts(conn).items())


def change(conn, item: str, amount: int, reason: str, by: int) -> None:
    reason = reason.strip()
    if item not in STONE_PER_UNIT or amount == 0 or not 1 <= len(reason) <= 120:
        raise ShipError("invalid_input")
    with transaction(conn):
        if counts(conn)[item] + amount < 0:
            raise ShipError("not_enough")
        if weight(conn) + amount * STONE_PER_UNIT[item] > CAPACITY_STONE:
            raise ShipError("overloaded")
        conn.execute(
            "INSERT INTO hold_changes (item, amount, reason, by_member_id, at) VALUES (?, ?, ?, ?, ?)",
            (item, amount, reason, by, now()),
        )
