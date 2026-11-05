---
kind: module
id: loot
type: domain
summary: The plunder. Money in reales, compensation for the wounded, and the split by shares.
code: []
tests: []
specs: []
depends_on: [voyage, crew, db]
updated: 2026-11-04
---

# loot — The Plunder

Gold! The sweetest word on the seven seas, and the one that starts the most fights. So the articles are exact: the wounded are paid first, then every real is divided by shares, and whatever won't divide goes to the chest. No fractions, no floats, no arguments.

> Planned code: `backend/app/loot/`. Planned tests: `backend/tests/loot/`.

## Functional rules
- **FR-01** Plunder is recorded in whole reales (an integer greater than 0) with a description, against the voyage in `plundering` (voyage#FR-05). A voyage can have many plunder entries.
- **FR-02** All money is stored and computed in whole reales. 1 doubloon = 32 reales, 1 piece of eight = 8 reales. Doubloons and pieces of eight are display units only.
- **FR-03** Compensation for an injury on the voyage, paid to a member of its roster:

| Injury | Pieces of eight | Reales |
|---|---|---|
| right_arm | 600 | 4800 |
| left_arm | 500 | 4000 |
| right_leg | 500 | 4000 |
| left_leg | 400 | 3200 |
| eye | 100 | 800 |
| finger | 100 | 800 |

- **FR-04** The split runs once per voyage, when it is `returning` or `docked`, and is final: it cannot be run again or changed. A correction is a new task.
- **FR-05** The split, in this order:
  1. Total plunder = sum of the voyage's plunder entries.
  2. Compensation is paid first. If the total compensation is more than the total plunder, the split is refused with `plunder_too_small` and nothing is recorded.
  3. The rest is divided by the roster's quarter-shares (voyage#FR-04): each member gets `rest * their_quarter_shares // total_quarter_shares` reales.
  4. The reales left over after step 3 go to the chest.
- **FR-06** A member who is `dead` at split time still gets their line; it is paid to the chest. A `marooned` member gets 0 and their part goes to the chest.
- **FR-07** A recorded split has one line per roster member (member, quarter-shares, compensation, share, total), and the sum of all lines plus the chest part equals the total plunder, to the real.
- **FR-08** Display: an amount in reales is shown as doubloons and reales, `<amount // 32> doubloons, <amount % 32> reales` (`4801` → `150 doubloons, 1 real`). Singular for 1.
- **FR-09** Only the quartermaster records plunder and compensation and runs the split (auth#FR-09).

## Contracts / interfaces
- `record_plunder(voyage_id, reales, description, by)`.
- `record_compensation(voyage_id, crew_member_id, injury, by)`.
- `split(voyage_id, by) -> Split` — FR-04 to FR-07.
- `chest_balance() -> int` (reales).
- `format_reales(amount: int) -> str` — FR-08. The deck uses the API's formatted string or the same rule.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet. The rule "money is whole reales" (FR-02) is planned as ADR-0001.

## Known pitfalls
- `//` rounds down for positive numbers only. Amounts here are never negative; if one is, stop: something upstream is broken.
- Check FR-07 inside the split itself and refuse to record if it fails. A split that loses one real is a mutiny.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
