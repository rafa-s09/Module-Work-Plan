import pytest

from app import crew, loot, voyage
from app.errors import ShipError


def refused(code, call, *args):
    with pytest.raises(ShipError) as error:
        call(*args)
    assert error.value.code == code


def come_home(conn, voyage_id, captain):
    voyage.change_state(conn, voyage_id, "returning", captain)


def lines_by_name(conn, voyage_id):
    return {line["name"]: line for line in loot.split_lines(conn, loot.get_split(conn, voyage_id)["id"])}


@pytest.mark.parametrize(
    "amount, shown",
    [
        (4801, "150 doubloons, 1 real"),
        (32, "1 doubloon, 0 reales"),
        (33, "1 doubloon, 1 real"),
        (5, "0 doubloons, 5 reales"),
    ],
)
def test_display(amount, shown):
    """TEST-0039: money is shown as doubloons and reales, singular for 1."""
    assert loot.format_reales(amount) == shown


def test_plunder_only_while_plundering(conn, stocked):
    """TEST-0040: plunder is whole reales above 0, recorded only while `plundering`."""
    voyage_id = voyage.create(conn, "Main", "Nassau", stocked["captain"])
    refused("not_plundering", loot.record_plunder, conn, voyage_id, 100, "too soon", stocked["quartermaster"])
    voyage.change_state(conn, voyage_id, "sailing", stocked["captain"])
    voyage.change_state(conn, voyage_id, "plundering", stocked["captain"])
    refused("invalid_input", loot.record_plunder, conn, voyage_id, 0, "nothing", stocked["quartermaster"])
    loot.record_plunder(conn, voyage_id, 100, "a galleon", stocked["quartermaster"])
    loot.record_plunder(conn, voyage_id, 50, "her cook's savings", stocked["quartermaster"])
    assert [p["amount_reales"] for p in loot.plunder_entries(conn, voyage_id)] == [100, 50]


@pytest.mark.parametrize(
    "injury, reales",
    [
        ("right_arm", 4800),
        ("left_arm", 4000),
        ("right_leg", 4000),
        ("left_leg", 3200),
        ("eye", 800),
        ("finger", 800),
    ],
)
def test_compensation_table(conn, stocked, at_prize, injury, reales):
    """TEST-0041: each injury pays its amount from the articles."""
    loot.record_compensation(conn, at_prize, stocked["sailor"], injury, stocked["quartermaster"])
    assert loot.compensation_entries(conn, at_prize)[0]["amount_reales"] == reales


def test_compensation_only_for_the_roster(conn, stocked, at_prize):
    """TEST-0042: compensation goes only to the voyage's roster, for a known injury."""
    late = crew.add_member(conn, "Late", "sailor")
    refused("not_on_roster", loot.record_compensation, conn, at_prize, late, "eye", stocked["quartermaster"])
    refused("invalid_input", loot.record_compensation, conn, at_prize, stocked["sailor"], "pride", stocked["quartermaster"])


def test_the_split(conn, stocked, at_prize):
    """TEST-0043: the wounded are paid first, the rest goes by quarter-shares, the leftover to the chest."""
    qm = stocked["quartermaster"]
    loot.record_plunder(conn, at_prize, 10_003, "a galleon", qm)
    loot.record_compensation(conn, at_prize, stocked["sailor"], "eye", qm)
    come_home(conn, at_prize, stocked["captain"])
    loot.split(conn, at_prize, qm)

    lines = lines_by_name(conn, at_prize)
    assert {n: (x["compensation_reales"], x["share_reales"], x["total_reales"]) for n, x in lines.items()} == {
        "Anne": (0, 3681, 3681),
        "Mary": (0, 2760, 2760),
        "Jack": (800, 1840, 2640),
        "Tim": (0, 920, 920),
    }
    row = loot.get_split(conn, at_prize)
    assert (row["plunder_reales"], row["compensation_reales"], row["leftover_reales"]) == (10_003, 800, 2)
    assert loot.chest_balance(conn) == 2


def test_compensation_over_plunder_is_refused(conn, stocked, at_prize):
    """TEST-0044: compensation over the plunder refuses the split and records nothing."""
    qm = stocked["quartermaster"]
    loot.record_plunder(conn, at_prize, 799, "a rowboat", qm)
    loot.record_compensation(conn, at_prize, stocked["sailor"], "eye", qm)
    come_home(conn, at_prize, stocked["captain"])
    refused("plunder_too_small", loot.split, conn, at_prize, qm)
    assert loot.get_split(conn, at_prize) is None


def test_split_is_final_and_timed(conn, stocked, at_prize):
    """TEST-0045: the split runs only on the way home or in port, once, and closes compensation."""
    qm = stocked["quartermaster"]
    loot.record_plunder(conn, at_prize, 100, "a chest", qm)
    refused("bad_split_time", loot.split, conn, at_prize, qm)
    come_home(conn, at_prize, stocked["captain"])
    loot.split(conn, at_prize, qm)
    refused("already_split", loot.split, conn, at_prize, qm)
    refused("already_split", loot.record_compensation, conn, at_prize, stocked["sailor"], "eye", qm)


def test_dead_and_marooned_pay_the_chest(conn, stocked, at_prize):
    """TEST-0046: lines of dead and marooned members are paid to the chest; ashore members still get theirs."""
    qm, captain = stocked["quartermaster"], stocked["captain"]
    loot.record_plunder(conn, at_prize, 2000, "a sloop", qm)
    loot.record_compensation(conn, at_prize, stocked["sailor"], "finger", qm)
    come_home(conn, at_prize, captain)
    crew.change_status(conn, stocked["sailor"], "dead", captain)
    crew.change_status(conn, stocked["cabin_boy"], "marooned", captain)
    loot.split(conn, at_prize, qm)

    lines = lines_by_name(conn, at_prize)
    assert lines["Jack"]["paid_to"] == "chest"
    assert lines["Tim"]["paid_to"] == "chest"
    assert lines["Anne"]["paid_to"] == "member"
    # rest 1200: Jack 240 + 800 compensation, Tim 120, no leftover.
    assert loot.chest_balance(conn) == 240 + 800 + 120


@pytest.mark.parametrize("total", [1, 19, 20, 21, 97, 1000, 10_003, 123_457])
def test_no_real_is_lost(conn, stocked, at_prize, total):
    """TEST-0047: the lines plus the leftover always equal the plunder, to the real."""
    qm = stocked["quartermaster"]
    loot.record_plunder(conn, at_prize, total, "whatever floats", qm)
    come_home(conn, at_prize, stocked["captain"])
    loot.split(conn, at_prize, qm)
    row = loot.get_split(conn, at_prize)
    assert sum(x["total_reales"] for x in loot.split_lines(conn, row["id"])) + row["leftover_reales"] == total
    assert loot.chest_balance(conn) == row["leftover_reales"]


def test_nothing_to_split(conn, stocked, at_prize):
    """TEST-0048: a voyage with no plunder has nothing to split."""
    come_home(conn, at_prize, stocked["captain"])
    refused("no_plunder", loot.split, conn, at_prize, stocked["quartermaster"])
