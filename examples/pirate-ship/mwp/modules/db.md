---
kind: module
id: db
type: infra
summary: The strongroom. One SQLite file, numbered migrations, nothing ever deleted.
code: []
tests: []
specs: []
depends_on: []
updated: 2026-11-04
---

# db — The Strongroom

Every name, every barrel and every real the ship has ever owned sleeps here, in one SQLite file, locked tight. No ORM, no cloud, no nonsense: plain `sqlite3` and plain SQL, the way the old sea dogs did it.

> Planned code: `backend/app/db/`, `backend/migrations/`. Planned tests: `backend/tests/db/`. Fill `code:` and `tests:` when the first task creates them.

## Functional rules
- **FR-01** The database is one SQLite file at the path in the environment variable `LEDGER_DB`, default `backend/data/ledger.db`. The folder is created if it does not exist.
- **FR-02** Every connection runs `PRAGMA foreign_keys = ON` and `PRAGMA journal_mode = WAL` before any other statement.
- **FR-03** Schema changes are SQL files in `backend/migrations/`, named `NNNN_slug.sql` (4 digits). At startup, files not yet in the table `schema_version` are applied in name order, each inside its own transaction, and recorded with their name and UTC time.
- **FR-04** A migration file that was applied is never edited. A change is a new file.
- **FR-05** Rows of `crew_members`, `voyages`, `splits` and `split_lines` are never deleted. A state or status column records what happened to them.
- **FR-06** `connect(":memory:")` gives a database with every migration applied. Tests use it, one per test.
- **FR-07** Money columns are `INTEGER` and hold reales. Their names end in `_reales` (`plunder_reales`).
- **FR-08** Times are stored as `TEXT` in UTC, ISO 8601 with seconds and a `Z` (`2026-11-04T18:30:00Z`).

## Non-functional rules
- **NFR-01** Applying every migration on an empty in-memory database takes under 1 second on the test machine.

## Contracts / interfaces
- `connect(path: str | None = None) -> sqlite3.Connection` — opens the file from FR-01 (or `path`), applies FR-02 and FR-03, sets `row_factory = sqlite3.Row`.
- `transaction(conn)` — context manager: commits on success, rolls back on any exception.
- Tables are owned by the module whose rules describe them: `accounts`, `sessions` (auth); `crew_members` (crew); `voyages`, `voyage_roster` (voyage); `plunder`, `compensations`, `splits`, `split_lines`, `chest` (loot); `hold_items` (hold).

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- SQLite forgets foreign keys on every new connection unless FR-02 runs. A connection opened by hand, without `connect()`, sails without them.
- `datetime.utcnow()` returns a naive time. Use `datetime.now(timezone.utc)`.
- WAL mode leaves `-wal` and `-shm` files beside the database. They belong to it; never delete them while the app runs.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
