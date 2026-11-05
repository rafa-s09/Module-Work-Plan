---
kind: project
mwp_version: 2
name: The Black Ledger
language: en
---

# The Black Ledger

## What it is
Arrr, the book that runs a pirate ship. The captain and the quartermaster use it to keep the crew roster, take the ship from port to plunder and back, mind what lies in the hold, and split every last doubloon by the articles, so no scallywag can cry he was cheated.

## Stack
- Back: Python 3.12, FastAPI, the standard `sqlite3` module.
- Front: Angular (standalone components), TypeScript.
- Database: SQLite, one file.
- Tests: pytest (back) — `cd backend && pytest`; Angular test runner (front) — `cd frontend && npm test`.

## Layout
```
backend/
├── app/<module>/        # one Python package per domain or infra module
├── migrations/          # numbered SQL files (see db)
└── tests/<module>/
frontend/
└── src/app/             # the deck: screens, theme, API client
```

## How to run
- Install: `cd backend && pip install -r requirements.txt`; `cd frontend && npm install`
- Run: `cd backend && uvicorn app.main:app --reload`; `cd frontend && npm start`
- First captain: `cd backend && python -m app.auth.make_captain` (see auth#FR-07)
- Test everything: `cd backend && pytest`, then `cd frontend && npm test`
- Test one module: `cd backend && pytest tests/<module>`

## Out of scope
- Real money, real ships, real piracy.
- More than one ship per ledger.
- Sign-up from the login screen, password reset by email, single sign-on.
- Offline use of the deck.
