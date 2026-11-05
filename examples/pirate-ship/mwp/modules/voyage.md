---
kind: module
id: voyage
type: domain
summary: The course. A voyage's states from port to plunder and back, and who sails on it.
code: [backend/app/voyage, backend/migrations/0004_voyage.sql]
tests: [backend/tests/voyage]
specs: []
depends_on: [crew, hold, db]
updated: 2026-11-05
---

# voyage — The Course

Hoist the sails! A voyage leaves port, finds a fat merchant, takes what's hers and comes home. Or sinks. Only the captain turns the wheel, and only one voyage sails at a time, for we have but one ship.

## Functional rules
- **FR-01** A voyage has a name (1 to 80 characters), a destination (1 to 80 characters), a state and the time of each state change. Tests: TEST-0036, TEST-0038
- **FR-02** At most one voyage is at sea (`sailing`, `plundering` or `returning`). While one is, creating a voyage or setting another one sail is refused with `ship_at_sea`. Tests: TEST-0033
- **FR-03** A voyage can set sail (`docked` → `sailing`) only if the hold has at least 1 `water` and at least 1 `rations` (hold#FR-01). Otherwise: `empty_hold`. Tests: TEST-0034
- **FR-04** When a voyage sets sail, the members `aboard` at that moment become its roster, each with their quarter-shares at that moment. The roster never changes after that. Tests: TEST-0035
- **FR-05** Plunder can be recorded only while the voyage is `plundering` (loot#FR-01). Tests: TEST-0040
- **FR-06** Only the captain creates voyages and changes the state (auth#FR-09). Tests: TEST-0055

## States and transitions
```
docked ──▶ sailing ──▶ plundering ──▶ returning ──▶ docked
              │             │              │
              └─────────────┴──────────────┴──▶ sunk
```
- `plundering` → `sailing` is also allowed: the prize got away, keep hunting.
- `sunk` is final. Its roster and any plunder already recorded stay in the ledger.
- A voyage that came back to `docked` is finished: setting sail again is refused. A new trip is a new voyage.
- Any other change is refused with `bad_course`.

Tests: TEST-0036, TEST-0037

## Contracts / interfaces
- `create(conn, name, destination, by) -> id` (starts `docked`).
- `change_state(conn, id, state, by)` — FR-02 to FR-04.
- `get`, `everything`, `active(conn) -> row | None`, `events(conn, id)`.
- `roster(conn, id) -> rows` of `crew_member_id`, `quarter_shares`, `name`, `rank`, `status`.
- `COURSES` — the allowed changes; the API sends them as `next_states`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0033 | FR-02 | One voyage at sea | backend/tests/voyage/test_voyage.py |
| TEST-0034 | FR-03 | No sailing with an empty hold | backend/tests/voyage/test_voyage.py |
| TEST-0035 | FR-04 | Roster and shares fixed at departure | backend/tests/voyage/test_voyage.py |
| TEST-0036 | FR-01, States | The whole course, with times | backend/tests/voyage/test_voyage.py |
| TEST-0037 | States | Bad courses, sunk is final, finished stays finished | backend/tests/voyage/test_voyage.py |
| TEST-0038 | FR-01 | Name and destination limits; unknown state | backend/tests/voyage/test_voyage.py |

## Related decisions
- None yet.

## Known pitfalls
- Read shares from the roster, never from `crew` today: a sailor promoted at sea is still a sailor for this voyage's split.
- A docked voyage may be new or finished. Having a roster is what tells them apart.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0004 | Built. FR-02 made exact, and a finished voyage cannot sail again. |
