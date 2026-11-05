---
kind: task
id: T-0004
status: done
size: M
created: 2026-11-05
modules: [hold, voyage]
reads: [crew, db]
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0004 — Set the course: the hold and the voyage states

## Goal
Fill the hold, hoist the sails. Build the cargo limits and the voyage states, with the roster fixed at departure.

## Scope
- Includes: hold changes and limits, voyage states and roster, migrations `0003` and `0004`.
- Excludes: plunder and split (T-0005).

## Acceptance criteria
- [x] AC-1 [auto: TEST-0029] Counts and weight.
- [x] AC-2 [auto: TEST-0030] Never over 2000 stone.
- [x] AC-3 [auto: TEST-0031] Never below 0.
- [x] AC-4 [auto: TEST-0032] Bad changes refused.
- [x] AC-5 [auto: TEST-0033] One voyage at sea.
- [x] AC-6 [auto: TEST-0034] No sailing with an empty hold.
- [x] AC-7 [auto: TEST-0035] Roster fixed at departure.
- [x] AC-8 [auto: TEST-0036, TEST-0037] The course and its refusals.
- [x] AC-9 [auto: TEST-0038] Input limits.
- [x] AC-10 [doc] `hold` and `voyage` modules and changelogs updated.

## Closure
- **Done:** The `hold` and `voyage` packages and their migrations. A finished voyage can no longer sail again, which the first wording of the module left open.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0029 to TEST-0038.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
