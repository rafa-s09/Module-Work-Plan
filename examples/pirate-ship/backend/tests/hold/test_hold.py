import pytest

from app import hold
from app.errors import ShipError


def refused(code, call, *args):
    with pytest.raises(ShipError) as error:
        call(*args)
    assert error.value.code == code


def test_counts_and_weight(conn, ship):
    """TEST-0029: counts are the sum of signed changes, and weight follows the stone table."""
    by = ship["quartermaster"]
    hold.change(conn, "rum", 10, "from Tortuga", by)
    hold.change(conn, "rum", -3, "a hard night", by)
    hold.change(conn, "powder", 2, "for the guns", by)
    hold.change(conn, "rations", 1, "hardtack", by)
    hold.change(conn, "water", 1, "fresh", by)
    assert hold.counts(conn) == {"rum": 7, "powder": 2, "rations": 1, "water": 1}
    assert hold.weight(conn) == 7 * 4 + 2 * 3 + 1 * 2 + 1 * 5


def test_never_overloaded(conn, ship):
    """TEST-0030: exactly 2000 stone is fine; one more is refused and nothing changes."""
    by = ship["quartermaster"]
    hold.change(conn, "rum", 500, "a very good day", by)
    assert hold.weight(conn) == 2000
    refused("overloaded", hold.change, conn, "rations", 1, "one more", by)
    assert hold.weight(conn) == 2000


def test_never_negative(conn, ship):
    """TEST-0031: a count below 0 is refused and nothing changes."""
    by = ship["quartermaster"]
    hold.change(conn, "rum", 2, "start", by)
    refused("not_enough", hold.change, conn, "rum", -3, "thirst", by)
    assert hold.counts(conn)["rum"] == 2


@pytest.mark.parametrize(
    "item, amount, reason",
    [("gold", 1, "no such item"), ("rum", 0, "nothing"), ("rum", 1, " "), ("rum", 1, "x" * 121)],
)
def test_bad_changes(conn, ship, item, amount, reason):
    """TEST-0032: unknown items, zero amounts and bad reasons are refused."""
    refused("invalid_input", hold.change, conn, item, amount, reason, ship["quartermaster"])
