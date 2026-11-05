---
kind: module
id: api
type: infra
summary: The speaking trumpet. FastAPI endpoints over the domain, with one shape for every answer and every error.
code: [backend/app/api, backend/app/main.py, backend/app/errors.py]
tests: [backend/tests/api]
specs: []
depends_on: [auth, crew, voyage, hold, loot]
updated: 2026-11-05
---

# api — The Speaking Trumpet

The trumpet carries orders from the deck to the ship and back. It checks who is shouting and what their rank may order, passes the order to the right module and shouts the answer back. Rules live in the domain, never here.

## Functional rules
- **FR-01** Every endpoint is under `/api` and speaks JSON. Money fields are integers in reales, named `*_reales`, each with a `*_display` string from loot#FR-08. Tests: TEST-0056
- **FR-02** Every endpoint except `POST /api/login` needs the header `Authorization: Bearer <token>` (auth#FR-03). Without a valid token: 401 `not_logged_in`. Tests: TEST-0050
- **FR-03** Errors always have the body `{"error": "<code>", "message": "<pirate message>"}`. The code is the domain's error code; the message is for humans and may change. Tests: TEST-0051
- **FR-04** Status codes: 400 invalid input and `weak_password` / `wrong_password`, 401 `bad_credentials` / `not_logged_in`, 403 `not_allowed`, 404 not found, 409 any other domain refusal (`rank_taken`, `ship_at_sea`, `overloaded`, ...), 423 `locked`, 500 anything else. Tests: TEST-0051, TEST-0052, TEST-0053
- **FR-05** A 500 never shows the exception or stack trace in the body. The code is `server_error`, the message `"Blimey! Something broke below deck."`, and the details go to the log. Tests: TEST-0054
- **FR-06** Endpoints. Each checks the order of auth#FR-09 before calling the domain. Tests: TEST-0049, TEST-0055, TEST-0056, TEST-0057

| Method and path | Does | Module |
|---|---|---|
| `POST /api/login`, `POST /api/logout` | Log in, log out | auth |
| `GET /api/me`, `PUT /api/me/password` | Who am I and my `orders`, change my password | auth |
| `POST /api/accounts`, `PUT /api/accounts/{member_id}/password` | Create an account, reset a password | auth |
| `GET /api/crew`, `POST /api/crew`, `PATCH /api/crew/{id}` | Roster, add, change rank or status | crew |
| `GET /api/voyages`, `POST /api/voyages`, `GET /api/voyages/{id}`, `POST /api/voyages/{id}/state` | List, create, read, change state | voyage |
| `GET /api/hold`, `POST /api/hold/changes` | Counts and weight, change an item | hold |
| `GET` and `POST /api/voyages/{id}/plunder`, `POST /api/voyages/{id}/compensations` | Read and record plunder, record compensation | loot |
| `POST` and `GET /api/voyages/{id}/split`, `GET /api/chest` | Run the split, read it, chest balance | loot |

## Non-functional rules
- **NFR-01** Every `GET` answers in under 200 ms with 1000 crew members and 100 voyages in the database, on the test machine. Tests: TEST-0058

## Contracts / interfaces
- The table in FR-06. FastAPI's `/docs` is generated from the code and is not a source of truth; this module is.
- `get_conn` opens one connection per request. Tests override it with an in-memory database.
- `errors.py` holds every error code and its message.
- No CORS: the deck reaches the API through the `ng serve` proxy, so both share one origin.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0049 | FR-06 | Login and `me` with orders | backend/tests/api/test_api.py |
| TEST-0050 | FR-02 | 401 without a valid token | backend/tests/api/test_api.py |
| TEST-0051 | FR-03, FR-04 | Status codes and body shape | backend/tests/api/test_api.py |
| TEST-0052 | FR-04 | Locked is 423 | backend/tests/api/test_api.py |
| TEST-0053 | FR-04 | Bad body is 400, never 422 | backend/tests/api/test_api.py |
| TEST-0054 | FR-05 | 500 hides the details | backend/tests/api/test_api.py |
| TEST-0055 | FR-06 | Orders enforced for every guarded endpoint | backend/tests/api/test_api.py |
| TEST-0056 | FR-01, FR-06 | A whole voyage, from creation to split | backend/tests/api/test_api.py |
| TEST-0057 | FR-06 | A status change through the API ends sessions | backend/tests/api/test_api.py |
| TEST-0058 | NFR-01 | GETs under 200 ms with a full ledger | backend/tests/api/test_api.py |

## Related decisions
- [ADR-0001 — All money is stored as whole reales](../decisions/ADR-0001-whole-reales.md)

## Known pitfalls
- FastAPI's default validation error is a 422 with its own body. `main.py` overrides it to give 400 with the FR-03 shape, and does the same for unknown routes.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0006 | Built. Added the endpoints the deck needs (`me` orders, password reset, voyage and plunder reads). CORS dropped in favor of the dev proxy. |
