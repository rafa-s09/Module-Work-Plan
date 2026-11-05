---
kind: module
id: api
type: infra
summary: The speaking trumpet. FastAPI endpoints over the domain, with one shape for every answer and every error.
code: []
tests: []
specs: []
depends_on: [auth, crew, voyage, hold, loot]
updated: 2026-11-04
---

# api — The Speaking Trumpet

The trumpet carries orders from the deck to the ship and back. It decides nothing: it checks who is shouting, passes the order to the right module and shouts the answer back. Rules live in the domain, never here.

> Planned code: `backend/app/main.py`, `backend/app/api/`. Planned tests: `backend/tests/api/`.

## Functional rules
- **FR-01** Every endpoint is under `/api` and speaks JSON. Money fields are integers in reales, named `*_reales`, each with a `*_display` string from loot#FR-08.
- **FR-02** Every endpoint except `POST /api/login` needs the header `Authorization: Bearer <token>` (auth#FR-03). Without a valid token: 401 `not_logged_in`.
- **FR-03** Errors always have the body `{"error": "<code>", "message": "<pirate message>"}`. The code is the domain's error code; the message is for humans and may change.
- **FR-04** Status codes: 400 invalid input, 401 `bad_credentials` / `not_logged_in`, 403 `not_allowed`, 404 not found, 409 any domain rule refused (`rank_taken`, `ship_at_sea`, `overloaded`, ...), 423 `locked`, 500 anything else.
- **FR-05** A 500 never shows the exception or stack trace in the body. The message is `"Blimey! Something broke below deck."` and the details go to the log.
- **FR-06** Endpoints:

| Method and path | Does | Module |
|---|---|---|
| `POST /api/login`, `POST /api/logout` | Log in, log out | auth |
| `GET /api/me`, `PUT /api/me/password` | Who am I, change my password | auth |
| `POST /api/accounts` | Create an account for a member | auth |
| `GET /api/crew`, `POST /api/crew`, `PATCH /api/crew/{id}` | Roster, add, change rank or status | crew |
| `GET /api/voyages`, `POST /api/voyages`, `POST /api/voyages/{id}/state` | List, create, change state | voyage |
| `GET /api/hold`, `POST /api/hold/changes` | Counts and weight, change an item | hold |
| `POST /api/voyages/{id}/plunder`, `POST /api/voyages/{id}/compensations` | Record plunder, compensation | loot |
| `POST /api/voyages/{id}/split`, `GET /api/voyages/{id}/split`, `GET /api/chest` | Run the split, read it, chest balance | loot |

## Non-functional rules
- **NFR-01** Every `GET` answers in under 200 ms with 1000 crew members and 100 voyages in the database, on the test machine.

## Contracts / interfaces
- The table in FR-06. FastAPI's `/docs` is generated from the code and is not a source of truth; this module is.
- CORS allows only the deck's origin, from the environment variable `DECK_ORIGIN` (default `http://localhost:4200`).

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- FastAPI's default validation error is a 422 with its own body. Override it to give 400 with the FR-03 shape.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
