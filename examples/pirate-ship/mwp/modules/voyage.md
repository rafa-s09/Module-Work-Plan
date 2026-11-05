---
kind: module
id: voyage
type: domain
summary: The course. A voyage's states from port to plunder and back, and who sails on it.
code: []
tests: []
specs: []
depends_on: [crew, hold, db]
updated: 2026-11-04
---

# voyage — The Course

Hoist the sails! A voyage leaves port, finds a fat merchant, takes what's hers and comes home. Or sinks. Only the captain turns the wheel, and only one voyage sails at a time, for we have but one ship.

> Planned code: `backend/app/voyage/`. Planned tests: `backend/tests/voyage/`.

## Functional rules
- **FR-01** A voyage has a name (1 to 80 characters), a destination (1 to 80 characters), a state and the time of each state change.
- **FR-02** At most one voyage is in a state other than `docked` or `sunk`. Creating a second one is refused with `ship_at_sea`.
- **FR-03** A voyage can set sail (`docked` → `sailing`) only if the hold has at least 1 `water` and at least 1 `rations` (hold#FR-01). Otherwise: `empty_hold`.
- **FR-04** When a voyage sets sail, the members `aboard` at that moment become its roster, each with their quarter-shares at that moment. The roster never changes after that.
- **FR-05** Plunder can be recorded only while the voyage is `plundering` (loot#FR-01).
- **FR-06** Only the captain changes the state (auth#FR-09).

## States and transitions
```
docked ──▶ sailing ──▶ plundering ──▶ returning ──▶ docked
              │             │              │
              └─────────────┴──────────────┴──▶ sunk
```
- `plundering` → `sailing` is also allowed: the prize got away, keep hunting.
- `sunk` is final. Its roster and any plunder already recorded stay in the ledger.
- A voyage that came back to `docked` is finished. A new trip is a new voyage.
- Any other change is refused with `bad_course`.

## Contracts / interfaces
- `create(name, destination) -> Voyage` (starts `docked`).
- `change_state(voyage_id, new_state, by)` — FR-02 to FR-06.
- `roster(voyage_id) -> list[(crew_member_id, quarter_shares)]`.
- `active() -> Voyage | None`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- Read shares from the roster, never from `crew` today: a sailor promoted at sea is still a sailor for this voyage's split.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
