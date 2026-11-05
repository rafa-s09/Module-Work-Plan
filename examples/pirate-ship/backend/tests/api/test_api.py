import time

import pytest

from app import crew, voyage

PASSWORD = "seven-seas-rum"


def test_login_and_me(client, boarded):
    """TEST-0049: login returns a token and who I am, with the orders my rank may give."""
    answer = client.post("/api/login", json={"username": "captain", "password": PASSWORD})
    assert answer.status_code == 200
    body = answer.json()
    assert body["me"]["name"] == "Anne"
    assert body["me"]["shares_display"] == "2"
    assert "move_ship" in body["me"]["orders"]
    assert "password" not in answer.text.replace("password_", "").lower()
    me = client.get("/api/me", headers={"Authorization": f"Bearer {body['token']}"})
    assert me.json()["id"] == body["me"]["id"]


@pytest.mark.parametrize(
    "method, path",
    [
        ("get", "/api/me"),
        ("post", "/api/logout"),
        ("get", "/api/crew"),
        ("get", "/api/voyages"),
        ("get", "/api/hold"),
        ("get", "/api/chest"),
    ],
)
def test_token_required(client, method, path):
    """TEST-0050: every endpoint but login answers 401 without a valid token."""
    for headers in ({}, {"Authorization": "Bearer nonsense"}, {"Authorization": "Basic abc"}):
        answer = client.request(method, path, headers=headers)
        assert answer.status_code == 401
        assert answer.json()["error"] == "not_logged_in"


def test_status_codes_and_error_shape(client, boarded):
    """TEST-0051: each refusal has its status code and the same body shape."""
    cases = [
        (client.post("/api/login", json={"username": "nobody", "password": "x"}), 401, "bad_credentials"),
        (client.post("/api/crew", json={"name": "B", "rank": "sailor"}, headers=boarded["sailor"]), 403, "not_allowed"),
        (client.patch("/api/crew/999", json={"rank": "gunner"}, headers=boarded["captain"]), 404, "not_found"),
        (client.post("/api/crew", json={"name": "B", "rank": "captain"}, headers=boarded["captain"]), 409, "rank_taken"),
        (client.get("/api/nowhere"), 404, "not_found"),
    ]
    for answer, status, code in cases:
        assert answer.status_code == status
        body = answer.json()
        assert body["error"] == code and body["message"] and set(body) == {"error", "message"}


def test_lock_status(client, boarded):
    """TEST-0052: a locked username answers 423."""
    for _ in range(5):
        client.post("/api/login", json={"username": "ghost", "password": "wrong"})
    answer = client.post("/api/login", json={"username": "ghost", "password": "wrong"})
    assert answer.status_code == 423 and answer.json()["error"] == "locked"


def test_invalid_input_is_400(client, boarded):
    """TEST-0053: a badly formed body answers 400 with the error shape, never 422."""
    answer = client.post("/api/crew", json={"name": 1}, headers=boarded["captain"])
    assert answer.status_code == 400 and answer.json()["error"] == "invalid_input"


def test_500_hides_the_details(client, boarded, monkeypatch):
    """TEST-0054: an unexpected error answers 500 with the fixed message and no details."""
    monkeypatch.setattr(crew, "everyone", lambda _: 1 / 0)
    answer = client.get("/api/crew", headers=boarded["captain"])
    assert answer.status_code == 500
    assert answer.json() == {"error": "server_error", "message": "Blimey! Something broke below deck."}


@pytest.mark.parametrize(
    "rank, method, path, body",
    [
        ("sailor", "post", "/api/accounts", {"crew_member_id": 1, "username": "abc", "password": "long-enough-1"}),
        ("quartermaster", "put", "/api/accounts/1/password", {"new_password": "long-enough-1"}),
        ("quartermaster", "post", "/api/crew", {"name": "B", "rank": "sailor"}),
        ("quartermaster", "patch", "/api/crew/3", {"rank": "gunner"}),
        ("quartermaster", "post", "/api/voyages", {"name": "A", "destination": "B"}),
        ("quartermaster", "post", "/api/voyages/1/state", {"state": "sailing"}),
        ("captain", "post", "/api/hold/changes", {"item": "rum", "amount": 1, "reason": "x"}),
        ("captain", "post", "/api/voyages/1/plunder", {"amount_reales": 1, "description": "x"}),
        ("captain", "post", "/api/voyages/1/compensations", {"crew_member_id": 1, "injury": "eye"}),
        ("captain", "post", "/api/voyages/1/split", None),
    ],
)
def test_orders_are_enforced(client, boarded, rank, method, path, body):
    """TEST-0055: the back refuses an order the rank may not give, whatever the deck shows."""
    answer = client.request(method, path, json=body, headers=boarded[rank])
    assert answer.status_code == 403 and answer.json()["error"] == "not_allowed"


def test_a_whole_voyage(client, boarded):
    """TEST-0056: a voyage from creation to split, with money as reales plus a display string."""
    captain, qm = boarded["captain"], boarded["quartermaster"]
    voyage_id = client.post("/api/voyages", json={"name": "Main", "destination": "Nassau"}, headers=captain).json()["id"]
    for state in ("sailing", "plundering"):
        assert client.post(f"/api/voyages/{voyage_id}/state", json={"state": state}, headers=captain).status_code == 200
    client.post(f"/api/voyages/{voyage_id}/plunder", json={"amount_reales": 10_003, "description": "galleon"}, headers=qm)
    jack = next(m for m in client.get("/api/crew", headers=qm).json() if m["name"] == "Jack")
    client.post(f"/api/voyages/{voyage_id}/compensations", json={"crew_member_id": jack["id"], "injury": "eye"}, headers=qm)
    client.post(f"/api/voyages/{voyage_id}/state", json={"state": "returning"}, headers=captain)

    made = client.post(f"/api/voyages/{voyage_id}/split", headers=qm)
    assert made.status_code == 201
    split = client.get(f"/api/voyages/{voyage_id}/split", headers=boarded["sailor"]).json()
    assert split == made.json()
    assert split["plunder_reales"] == 10_003
    assert split["plunder_display"] == "312 doubloons, 19 reales"
    line = next(x for x in split["lines"] if x["name"] == "Jack")
    assert (line["total_reales"], line["total_display"]) == (2640, "82 doubloons, 16 reales")
    assert client.get("/api/chest", headers=qm).json() == {"balance_reales": 2, "balance_display": "0 doubloons, 2 reales"}


def test_leaving_the_ship_through_the_api_ends_sessions(client, boarded, stocked):
    """TEST-0057: changing a status through the API ends that member's sessions."""
    answer = client.patch(f"/api/crew/{stocked['sailor']}", json={"status": "ashore"}, headers=boarded["captain"])
    assert answer.json()["status"] == "ashore"
    assert client.get("/api/me", headers=boarded["sailor"]).status_code == 401


def test_gets_are_fast(conn, client, boarded, stocked):
    """TEST-0058: every GET answers in under 200 ms with 1000 crew members and 100 voyages."""
    conn.executemany(
        "INSERT INTO crew_members (name, rank, signed_at) VALUES (?, 'sailor', 'x')",
        [(f"Sailor {i}",) for i in range(1000)],
    )
    for i in range(100):
        voyage.create(conn, f"Voyage {i}", "Nassau", stocked["captain"])
    for path in ("/api/me", "/api/crew", "/api/voyages", "/api/hold", "/api/chest"):
        start = time.perf_counter()
        assert client.get(path, headers=boarded["sailor"]).status_code == 200
        assert time.perf_counter() - start < 0.2, path
