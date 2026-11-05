---
kind: module
id: crew
type: domain
summary: The ship's articles. Who sails, at what rank, for how many shares, and what became of them.
code: [backend/app/crew, backend/migrations/0001_crew.sql]
tests: [backend/tests/crew]
specs: []
depends_on: [db]
updated: 2026-11-05
---

# crew — The Ship's Articles

Every soul aboard signed the articles, and the articles say what each one is worth. The captain takes the most, the cabin boy the least, and nobody, NOBODY, is ever struck from the book. Not even the ones Davy Jones took.

## Functional rules
- **FR-01** A crew member has a name (1 to 60 characters), a rank, a status and the date they signed the articles. Names are unique among members with status `aboard`, ignoring case; otherwise `name_taken`. Tests: TEST-0009
- **FR-02** Ranks and their shares, stored in quarter-shares. Tests: TEST-0008, TEST-0015

| Rank | Shares | Quarter-shares |
|---|---|---|
| captain | 2 | 8 |
| quartermaster | 1½ | 6 |
| boatswain | 1¼ | 5 |
| gunner | 1¼ | 5 |
| sailor | 1 | 4 |
| cabin_boy | ½ | 2 |

- **FR-03** There is at most one `captain` and one `quartermaster` with status `aboard`. A change that breaks this is refused with `rank_taken`, and so is a captain aboard stepping down. Making a new captain is one order that also sets the old captain to `sailor`; the new one must be aboard (`not_aboard`). Tests: TEST-0010, TEST-0011
- **FR-04** A member's status is one of `aboard`, `ashore`, `marooned`, `dead`. New members start `aboard`. Tests: TEST-0009
- **FR-05** Status changes: `aboard` → `ashore`, `marooned` or `dead`; `ashore` → `aboard`. `marooned` and `dead` are final. Any other change is refused with `bad_status_change`. Tests: TEST-0012
- **FR-06** The captain cannot leave `aboard` while captain (`captain_required`). Make another captain first. Tests: TEST-0013
- **FR-07** A rank or status change while a voyage is at sea does not change that voyage's roster or its shares. See voyage#FR-04. Tests: TEST-0035
- **FR-08** Every rank or status change is recorded with the member, the old and new value, who ordered it and when. Tests: TEST-0014

## Contracts / interfaces
- `quarter_shares(rank) -> int` — FR-02. The only place share values live.
- `shares_display(quarters) -> str` — `5` → `1¼`.
- `get(conn, id)`, `everyone(conn)`, `aboard(conn)`.
- `add_member(conn, name, rank) -> id`, `change_rank(conn, id, rank, by)`, `change_status(conn, id, status, by)`. The API changes status through `auth.change_status`, which also ends sessions (auth#FR-08).

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0008 | FR-02 | Quarter-shares of each rank | backend/tests/crew/test_crew.py |
| TEST-0009 | FR-01, FR-04 | Names unique aboard, free after leaving; bad names and ranks refused | backend/tests/crew/test_crew.py |
| TEST-0010 | FR-03 | A second officer refused; captain cannot step down alone | backend/tests/crew/test_crew.py |
| TEST-0011 | FR-03 | New captain demotes the old one | backend/tests/crew/test_crew.py |
| TEST-0012 | FR-05 | Allowed and refused status changes | backend/tests/crew/test_crew.py |
| TEST-0013 | FR-06 | Captain cannot leave | backend/tests/crew/test_crew.py |
| TEST-0014 | FR-08 | Changes recorded | backend/tests/crew/test_crew.py |
| TEST-0015 | FR-02 | Shares for humans | backend/tests/crew/test_crew.py |

## Related decisions
- None yet.

## Known pitfalls
- Shares are never floats. 1¼ is 5 quarter-shares, full stop. A float here turns the split into a mutiny (see loot).
- `dead` is a status, not a delete. The split history needs the name forever (db#FR-05).

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0002 | Built. Added `name_taken`, `not_aboard` and `captain_required`; a captain cannot step down alone. |
