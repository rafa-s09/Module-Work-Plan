---
kind: module
id: hold
type: domain
summary: Below deck. Rum, powder, rations and water, in whole units, never past what the ship can carry.
code: []
tests: []
specs: []
depends_on: [db]
updated: 2026-11-04
---

# hold — Below Deck

Yo ho ho and a barrel of rum. Or forty. The hold carries what keeps the crew alive and the cannons loud, and she sinks if ye overload her, so the ledger counts every keg.

> Planned code: `backend/app/hold/`. Planned tests: `backend/tests/hold/`.

## Functional rules
- **FR-01** The hold has four items, counted in whole units. Weight per unit in stone:

| Item | Unit | Stone per unit |
|---|---|---|
| rum | barrel | 4 |
| powder | keg | 3 |
| rations | crate | 2 |
| water | cask | 5 |

- **FR-02** The total weight of the hold is never over 2000 stone. A change that would pass it is refused with `overloaded`, and nothing changes.
- **FR-03** A count is never below 0. A change that would make it negative is refused with `not_enough`, and nothing changes.
- **FR-04** Every change is a signed amount for one item (`+10 rum`, `-3 powder`) with a reason (1 to 120 characters), who made it and when. The current count is the sum of the changes.
- **FR-05** Only the quartermaster and the boatswain change the hold (auth#FR-09).

## Contracts / interfaces
- `counts() -> dict[item, int]`, `weight() -> int` (stone), `capacity() -> 2000`.
- `change(item, amount, reason, by)` — raises `overloaded`, `not_enough`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- Two changes at once can both pass FR-02 alone and break it together. Check and write inside one transaction (db `transaction`).

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
