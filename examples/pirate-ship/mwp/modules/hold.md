---
kind: module
id: hold
type: domain
summary: Below deck. Rum, powder, rations and water, in whole units, never past what the ship can carry.
code: [backend/app/hold, backend/migrations/0003_hold.sql]
tests: [backend/tests/hold]
specs: []
depends_on: [db]
updated: 2026-11-05
---

# hold — Below Deck

Yo ho ho and a barrel of rum. Or forty. The hold carries what keeps the crew alive and the cannons loud, and she sinks if ye overload her, so the ledger counts every keg.

## Functional rules
- **FR-01** The hold has four items, counted in whole units. Weight per unit in stone. Tests: TEST-0029

| Item | Unit | Stone per unit |
|---|---|---|
| rum | barrel | 4 |
| powder | keg | 3 |
| rations | crate | 2 |
| water | cask | 5 |

- **FR-02** The total weight of the hold is never over 2000 stone. A change that would pass it is refused with `overloaded`, and nothing changes. Tests: TEST-0030
- **FR-03** A count is never below 0. A change that would make it negative is refused with `not_enough`, and nothing changes. Tests: TEST-0031
- **FR-04** Every change is a signed, non-zero amount for one item (`+10 rum`, `-3 powder`) with a reason (1 to 120 characters), who made it and when. The current count is the sum of the changes. Tests: TEST-0029, TEST-0032
- **FR-05** Only the quartermaster and the boatswain change the hold (auth#FR-09). Tests: TEST-0055

## Contracts / interfaces
- `counts(conn) -> dict[item, int]`, `weight(conn) -> int` (stone), `CAPACITY_STONE = 2000`.
- `change(conn, item, amount, reason, by)` — raises `overloaded`, `not_enough`, `invalid_input`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0029 | FR-01, FR-04 | Counts are the sum of changes; weight follows the table | backend/tests/hold/test_hold.py |
| TEST-0030 | FR-02 | Exactly 2000 stone is fine; one more is refused | backend/tests/hold/test_hold.py |
| TEST-0031 | FR-03 | Below 0 is refused | backend/tests/hold/test_hold.py |
| TEST-0032 | FR-04 | Unknown item, zero amount, bad reason refused | backend/tests/hold/test_hold.py |

## Related decisions
- None yet.

## Known pitfalls
- Two changes at once can both pass FR-02 alone and break it together. Check and write inside one `transaction`, as `change` does.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0004 | Built. |
