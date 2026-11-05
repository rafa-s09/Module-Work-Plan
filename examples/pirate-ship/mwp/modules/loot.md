---
kind: module
id: loot
type: domain
summary: The plunder. Money in reales, compensation for the wounded, and the split by shares.
code: [backend/app/loot, backend/migrations/0005_loot.sql]
tests: [backend/tests/loot]
specs: []
depends_on: [voyage, db]
updated: 2026-11-05
---

# loot — The Plunder

Gold! The sweetest word on the seven seas, and the one that starts the most fights. So the articles are exact: the wounded are paid first, then every real is divided by shares, and whatever won't divide goes to the chest. No fractions, no floats, no arguments.

## Functional rules
- **FR-01** Plunder is recorded in whole reales (an integer greater than 0) with a description (1 to 120 characters), against a voyage in `plundering`; otherwise `not_plundering`. A voyage can have many plunder entries. Tests: TEST-0040
- **FR-02** All money is stored and computed in whole reales. 1 doubloon = 32 reales, 1 piece of eight = 8 reales. Doubloons and pieces of eight are display units only (ADR-0001). Tests: TEST-0039
- **FR-03** Compensation for an injury, paid to a member of the voyage's roster (`not_on_roster`), recorded until the split. Tests: TEST-0041, TEST-0042

| Injury | Pieces of eight | Reales |
|---|---|---|
| right_arm | 600 | 4800 |
| left_arm | 500 | 4000 |
| right_leg | 500 | 4000 |
| left_leg | 400 | 3200 |
| eye | 100 | 800 |
| finger | 100 | 800 |

- **FR-04** The split runs once per voyage, when it is `returning` or `docked` (`bad_split_time`), and is final: it cannot be run again or changed (`already_split`). A correction is a new task. Tests: TEST-0045
- **FR-05** The split, in this order. Tests: TEST-0043, TEST-0044, TEST-0048
  1. Total plunder = sum of the voyage's plunder entries. With nothing to divide, it is refused with `no_plunder`.
  2. Compensation is paid first. If the total compensation is more than the total plunder, the split is refused with `plunder_too_small` and nothing is recorded.
  3. The rest is divided by the roster's quarter-shares (voyage#FR-04): each member gets `rest * their_quarter_shares // total_quarter_shares` reales.
  4. The reales left over after step 3 go to the chest.
- **FR-06** The line of a member who is `dead` or `marooned` at split time is still computed in full, compensation included, but `paid_to` is `chest` instead of `member`. Members `ashore` are paid. Tests: TEST-0046
- **FR-07** A recorded split has one line per roster member (member, quarter-shares, compensation, share, total, paid_to), and the sum of all line totals plus the leftover equals the total plunder, to the real. The split checks this and refuses with `split_broken` before recording. Tests: TEST-0047
- **FR-08** Display: an amount in reales is shown as doubloons and reales, `<amount // 32> doubloons, <amount % 32> reales` (`4801` → `150 doubloons, 1 real`). Singular for 1. Tests: TEST-0039
- **FR-09** Only the quartermaster records plunder and compensation and runs the split (auth#FR-09). Tests: TEST-0055

## Contracts / interfaces
- `record_plunder(conn, voyage_id, reales, description, by)`.
- `record_compensation(conn, voyage_id, member_id, injury, by)`.
- `split(conn, voyage_id, by)` — FR-04 to FR-07. `get_split`, `split_lines`.
- `chest_balance(conn) -> int` — leftovers plus lines paid to the chest.
- `format_reales(amount) -> str` — FR-08. The deck shows the API's string and never computes it.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0039 | FR-02, FR-08 | Doubloons and reales, singular for 1 | backend/tests/loot/test_loot.py |
| TEST-0040 | FR-01 | Plunder only while plundering, above 0 | backend/tests/loot/test_loot.py |
| TEST-0041 | FR-03 | Each injury pays its amount | backend/tests/loot/test_loot.py |
| TEST-0042 | FR-03 | Only for the roster, known injuries | backend/tests/loot/test_loot.py |
| TEST-0043 | FR-05 | 10003 reales, one eye: exact lines, 2 reales to the chest | backend/tests/loot/test_loot.py |
| TEST-0044 | FR-05 | Compensation over the plunder refused | backend/tests/loot/test_loot.py |
| TEST-0045 | FR-04 | Timing; once only; closes compensation | backend/tests/loot/test_loot.py |
| TEST-0046 | FR-06 | Dead and marooned pay the chest | backend/tests/loot/test_loot.py |
| TEST-0047 | FR-07 | Lines plus leftover equal the plunder, for many totals | backend/tests/loot/test_loot.py |
| TEST-0048 | FR-05 | Nothing to split | backend/tests/loot/test_loot.py |

## Related decisions
- [ADR-0001 — All money is stored as whole reales](../decisions/ADR-0001-whole-reales.md)

## Known pitfalls
- `//` rounds down for positive numbers only. Amounts here are never negative; if one is, stop: something upstream is broken.
- A dead member's money appears in their line and in the chest balance. Add line totals to the leftover to check a split, not to the chest balance.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0005 | Built. FR-06 and FR-07 made exact with `paid_to`, since the first wording counted a dead member's money twice. Added `no_plunder`. |
