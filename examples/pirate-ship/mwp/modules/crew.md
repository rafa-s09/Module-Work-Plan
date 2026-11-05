---
kind: module
id: crew
type: domain
summary: The ship's articles. Who sails, at what rank, for how many shares, and what became of them.
code: []
tests: []
specs: []
depends_on: [db]
updated: 2026-11-04
---

# crew — The Ship's Articles

Every soul aboard signed the articles, and the articles say what each one is worth. The captain takes the most, the cabin boy the least, and nobody, NOBODY, is ever struck from the book. Not even the ones Davy Jones took.

> Planned code: `backend/app/crew/`. Planned tests: `backend/tests/crew/`.

## Functional rules
- **FR-01** A crew member has a name (1 to 60 characters), a rank, a status and the date they signed the articles. Names are unique among members with status `aboard`, ignoring case.
- **FR-02** Ranks and their shares, stored in quarter-shares:

| Rank | Shares | Quarter-shares |
|---|---|---|
| captain | 2 | 8 |
| quartermaster | 1½ | 6 |
| boatswain | 1¼ | 5 |
| gunner | 1¼ | 5 |
| sailor | 1 | 4 |
| cabin_boy | ½ | 2 |

- **FR-03** There is exactly one `captain` and at most one `quartermaster` with status `aboard`. A change that breaks this is refused with `rank_taken`. Making a new captain is one order that also sets the old captain to `sailor`.
- **FR-04** A member's status is one of `aboard`, `ashore`, `marooned`, `dead`. New members start `aboard`.
- **FR-05** Status changes: `aboard` → `ashore`, `marooned` or `dead`; `ashore` → `aboard`. `marooned` and `dead` are final. Any other change is refused with `bad_status_change`. Leaving `aboard` ends the member's sessions (auth#FR-08).
- **FR-06** The captain cannot leave `aboard` while captain. Make another captain first.
- **FR-07** A rank or status change while a voyage is not `docked` does not change that voyage's roster or its shares (voyage#FR-04).
- **FR-08** Every rank or status change is recorded with the member, the old and new value, who ordered it and when.

## Contracts / interfaces
- `quarter_shares(rank) -> int` — FR-02. The only place share values live.
- `aboard() -> list[CrewMember]`.
- `add_member`, `change_rank`, `change_status` — raise the error codes above.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- Shares are never floats. 1¼ is 5 quarter-shares, full stop. A float here turns the split into a mutiny (see loot).
- `dead` is a status, not a delete. The split history needs the name forever (db#FR-05).

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
