import pytest
from fastapi.testclient import TestClient

from app import auth, crew, db, hold, voyage
from app.api import get_conn
from app.main import create_app

PASSWORD = "seven-seas-rum"


@pytest.fixture
def conn():
    connection = db.connect(":memory:")
    yield connection
    connection.close()


@pytest.fixture
def ship(conn):
    """A small crew: quarter-shares 8, 6, 4, 2. Returns their ids by rank."""
    return {
        rank: crew.add_member(conn, name, rank)
        for name, rank in [
            ("Anne", "captain"),
            ("Mary", "quartermaster"),
            ("Jack", "sailor"),
            ("Tim", "cabin_boy"),
        ]
    }


@pytest.fixture
def stocked(conn, ship):
    hold.change(conn, "water", 10, "fresh casks", ship["quartermaster"])
    hold.change(conn, "rations", 10, "hardtack", ship["quartermaster"])
    return ship


@pytest.fixture
def at_prize(conn, stocked):
    """A voyage in `plundering`, with its roster fixed."""
    voyage_id = voyage.create(conn, "Spanish Main", "Nassau", stocked["captain"])
    voyage.change_state(conn, voyage_id, "sailing", stocked["captain"])
    voyage.change_state(conn, voyage_id, "plundering", stocked["captain"])
    return voyage_id


@pytest.fixture
def client(conn):
    app = create_app()
    app.dependency_overrides[get_conn] = lambda: conn
    return TestClient(app, raise_server_exceptions=False)


@pytest.fixture
def boarded(conn, client, stocked):
    """Accounts for the whole small crew. Maps a rank to its auth headers."""
    headers = {}
    for rank, member_id in stocked.items():
        auth.create_account(conn, member_id, rank, PASSWORD)
        headers[rank] = {"Authorization": f"Bearer {auth.login(conn, rank, PASSWORD)}"}
    return headers
