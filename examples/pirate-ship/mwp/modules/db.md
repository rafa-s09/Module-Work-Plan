---
kind: module
id: db
type: infra
summary: The strongroom. One SQLite file, numbered migrations, nothing ever deleted.
code: [backend/app/db, backend/migrations]
tests: [backend/tests/db]
specs: []
depends_on: []
updated: 2026-11-05
---

# db — The Strongroom

Every name, every barrel and every real the ship has ever owned sleeps here, in one SQLite file, locked tight. No ORM, no cloud, no nonsense: plain `sqlite3` and plain SQL, the way the old sea dogs did it.

## Functional rules
- **FR-01** The database is one SQLite file at the path in the environment variable `LEDGER_DB`, default `backend/data/ledger.db`. The folder is created if it does not exist. Tests: TEST-0005
- **FR-02** Every connection runs `PRAGMA foreign_keys = ON` and `PRAGMA journal_mode = WAL` before any other statement. Tests: TEST-0002, TEST-0005
- **FR-03** Schema changes are SQL files in `backend/migrations/`, named `NNNN_slug.sql` (4 digits). When a connection opens, files not yet in the table `schema_version` are applied in name order, each inside its own transaction, and recorded with their name and UTC time. Tests: TEST-0001, TEST-0003
- **FR-04** A migration file that was applied is never edited. A change is a new file.
- **FR-05** Rows of `crew_members`, `voyages`, `splits` and `split_lines` are never deleted. A state or status column records what happened to them.
- **FR-06** `connect(":memory:")` gives a database with every migration applied. Tests use it, one per test. Tests: TEST-0001
- **FR-07** Money columns are `INTEGER` and hold reales. Their names end in `_reales` (`plunder_reales`).
- **FR-08** Times are stored as `TEXT` in UTC, ISO 8601 with seconds and a `Z` (`2026-11-04T18:30:00Z`). Tests: TEST-0007

## Non-functional rules
- **NFR-01** Applying every migration on an empty in-memory database takes under 1 second on the test machine. Tests: TEST-0006

## Contracts / interfaces
- `connect(path=None, migrations=MIGRATIONS) -> sqlite3.Connection` — opens the file from FR-01 (or `path`), applies FR-02 and FR-03, sets `row_factory = sqlite3.Row`. Autocommit: only `transaction` opens transactions.
- `transaction(conn)` — context manager: commits on success, rolls back on any exception. Inside another transaction, it joins it.
- `now() -> str`, `to_text(datetime) -> str` — FR-08.
- Tables are owned by the module whose rules describe them: `accounts`, `sessions`, `login_failures` (auth); `crew_members`, `crew_changes` (crew); `voyages`, `voyage_events`, `voyage_roster` (voyage); `plunder`, `compensations`, `splits`, `split_lines`, `chest` (loot); `hold_changes` (hold).

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0001 | FR-03, FR-06 | Every migration applied once, recorded, and not applied twice | backend/tests/db/test_db.py |
| TEST-0002 | FR-02 | A row with a dead foreign key is refused | backend/tests/db/test_db.py |
| TEST-0003 | FR-03 | A failing migration leaves no table and no record | backend/tests/db/test_db.py |
| TEST-0004 | Contracts | An exception undoes a transaction, including a joined one | backend/tests/db/test_db.py |
| TEST-0005 | FR-01, FR-02 | File comes from `LEDGER_DB`, folder created, WAL on | backend/tests/db/test_db.py |
| TEST-0006 | NFR-01 | All migrations in under 1 second | backend/tests/db/test_db.py |
| TEST-0007 | FR-08 | Time format | backend/tests/db/test_db.py |

## Related decisions
- [ADR-0001 — All money is stored as whole reales](../decisions/ADR-0001-whole-reales.md)

## Known pitfalls
- SQLite forgets foreign keys on every new connection unless FR-02 runs. A connection opened by hand, without `connect()`, sails without them.
- `datetime.utcnow()` returns a naive time. Use `datetime.now(UTC)`.
- WAL mode leaves `-wal` and `-shm` files beside the database. They belong to it; never delete them while the app runs.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0001 | Built. Migrations run when a connection opens. |
