import pytest

from app import crew, voyage
from app.errors import ShipError


def refused(code, call, *args):
    with pytest.raises(ShipError) as error:
        call(*args)
    assert error.value.code == code


@pytest.fixture
def docked(conn, stocked):
    return voyage.create(conn, "Spanish Main", "Nassau", stocked["captain"])


def test_one_voyage_at_sea(conn, stocked, docked):
    """TEST-0033: while a voyage is at sea, a second cannot be created or set sail."""
    other = voyage.create(conn, "Second", "Tortuga", stocked["captain"])
    voyage.change_state(conn, docked, "sailing", stocked["captain"])
    assert voyage.active(conn)["id"] == docked
    refused("ship_at_sea", voyage.create, conn, "Third", "Havana", stocked["captain"])
    refused("ship_at_sea", voyage.change_state, conn, other, "sailing", stocked["captain"])


def test_needs_water_and_rations(conn, ship):
    """TEST-0034: a voyage without water or rations in the hold cannot set sail."""
    voyage_id = voyage.create(conn, "Dry", "Nowhere", ship["captain"])
    refused("empty_hold", voyage.change_state, conn, voyage_id, "sailing", ship["captain"])


def test_roster_is_fixed_at_departure(conn, stocked, docked):
    """TEST-0035: the roster is who is aboard at departure, with their shares, and never changes."""
    crew.change_status(conn, stocked["cabin_boy"], "ashore", stocked["captain"])
    voyage.change_state(conn, docked, "sailing", stocked["captain"])
    crew.change_rank(conn, stocked["sailor"], "gunner", stocked["captain"])
    crew.add_member(conn, "Late", "sailor")
    assert [(r["name"], r["quarter_shares"]) for r in voyage.roster(conn, docked)] == [
        ("Anne", 8),
        ("Mary", 6),
        ("Jack", 4),
    ]


def test_the_course(conn, stocked, docked):
    """TEST-0036: a voyage goes the whole way, may keep hunting, and records each state with its time."""
    states = ["sailing", "plundering", "sailing", "plundering", "returning", "docked"]
    for state in states:
        voyage.change_state(conn, docked, state, stocked["captain"])
    events = voyage.events(conn, docked)
    assert [e["state"] for e in events] == ["docked", *states]
    assert all(e["at"] for e in events)


@pytest.mark.parametrize(
    "path, bad",
    [
        ([], "plundering"),
        ([], "docked"),
        (["sailing"], "returning"),
        (["sailing"], "docked"),
        (["sailing", "plundering"], "docked"),
        (["sailing", "plundering", "returning"], "plundering"),
        (["sailing", "sunk"], "sailing"),
        (["sailing", "plundering", "sunk"], "docked"),
        (["sailing", "plundering", "returning", "docked"], "sailing"),
    ],
)
def test_bad_courses(conn, stocked, docked, path, bad):
    """TEST-0037: changes off the chart are refused; sunk is final; a finished voyage stays finished."""
    for state in path:
        voyage.change_state(conn, docked, state, stocked["captain"])
    refused("bad_course", voyage.change_state, conn, docked, bad, stocked["captain"])


def test_bad_input(conn, stocked):
    """TEST-0038: names and destinations need 1 to 80 characters; unknown states are refused."""
    refused("invalid_input", voyage.create, conn, " ", "Nassau", stocked["captain"])
    refused("invalid_input", voyage.create, conn, "x" * 81, "Nassau", stocked["captain"])
    refused("invalid_input", voyage.change_state, conn, 1, "flying", stocked["captain"])
