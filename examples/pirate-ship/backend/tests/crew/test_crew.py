import pytest

from app import crew
from app.errors import ShipError


def refused(code, call, *args):
    with pytest.raises(ShipError) as error:
        call(*args)
    assert error.value.code == code


def test_shares_by_rank():
    """TEST-0008: each rank has the quarter-shares of the articles."""
    assert crew.QUARTER_SHARES == {
        "captain": 8,
        "quartermaster": 6,
        "boatswain": 5,
        "gunner": 5,
        "sailor": 4,
        "cabin_boy": 2,
    }


def test_names_unique_aboard(conn, ship):
    """TEST-0009: names are unique aboard ignoring case, and free again once a member leaves."""
    refused("name_taken", crew.add_member, conn, "jACK", "sailor")
    crew.change_status(conn, ship["sailor"], "ashore", ship["captain"])
    new_jack = crew.add_member(conn, "Jack", "sailor")
    assert crew.get(conn, new_jack)["status"] == "aboard"
    refused("name_taken", crew.change_status, conn, ship["sailor"], "aboard", ship["captain"])
    refused("invalid_input", crew.add_member, conn, " ", "sailor")
    refused("invalid_input", crew.add_member, conn, "x" * 61, "sailor")
    refused("invalid_input", crew.add_member, conn, "Bo", "admiral")


def test_one_captain_one_quartermaster(conn, ship):
    """TEST-0010: a second captain or quartermaster aboard is refused."""
    refused("rank_taken", crew.add_member, conn, "Bart", "captain")
    refused("rank_taken", crew.add_member, conn, "Bart", "quartermaster")
    refused("rank_taken", crew.change_rank, conn, ship["sailor"], "quartermaster", ship["captain"])
    refused("rank_taken", crew.change_rank, conn, ship["captain"], "sailor", ship["captain"])


def test_new_captain_replaces_old(conn, ship):
    """TEST-0011: making a new captain sets the old one to sailor, in one order."""
    crew.change_rank(conn, ship["sailor"], "captain", ship["captain"])
    assert crew.get(conn, ship["sailor"])["rank"] == "captain"
    assert crew.get(conn, ship["captain"])["rank"] == "sailor"


@pytest.mark.parametrize(
    "path, last, allowed",
    [
        (["ashore"], "aboard", True),
        (["marooned"], "aboard", False),
        (["dead"], "aboard", False),
        (["dead"], "ashore", False),
        (["ashore"], "dead", False),
        (["ashore"], "marooned", False),
    ],
)
def test_status_changes(conn, ship, path, last, allowed):
    """TEST-0012: only the status changes of the articles are allowed; marooned and dead are final."""
    for status in path:
        crew.change_status(conn, ship["sailor"], status, ship["captain"])
    if allowed:
        crew.change_status(conn, ship["sailor"], last, ship["captain"])
    else:
        refused("bad_status_change", crew.change_status, conn, ship["sailor"], last, ship["captain"])


def test_captain_cannot_leave(conn, ship):
    """TEST-0013: the captain cannot leave while captain."""
    refused("captain_required", crew.change_status, conn, ship["captain"], "dead", ship["captain"])


def test_changes_are_recorded(conn, ship):
    """TEST-0014: rank and status changes record old, new, who and when."""
    crew.change_rank(conn, ship["sailor"], "gunner", ship["captain"])
    crew.change_status(conn, ship["sailor"], "ashore", ship["captain"])
    rows = conn.execute("SELECT * FROM crew_changes ORDER BY id").fetchall()
    assert [(r["field"], r["old_value"], r["new_value"], r["by_member_id"]) for r in rows] == [
        ("rank", "sailor", "gunner", ship["captain"]),
        ("status", "aboard", "ashore", ship["captain"]),
    ]
    assert all(r["at"] for r in rows)


def test_shares_display():
    """TEST-0015: quarter-shares read as shares for humans."""
    assert [crew.shares_display(q) for q in (8, 6, 5, 4, 2)] == ["2", "1½", "1¼", "1", "½"]
