---
kind: task
id: T-0002
status: done
size: M
created: 2026-11-05
modules: [crew]
reads: [db]
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0002 — Sign the articles: ranks, shares and status

## Goal
Every soul aboard needs a rank, a share and a fate. Build the crew rules, so the split can trust them.

## Scope
- Includes: ranks, shares, status changes, the record of changes, migration `0001`.
- Excludes: accounts and sessions (T-0003).

## Acceptance criteria
- [x] AC-1 [auto: TEST-0008] Quarter-shares of each rank.
- [x] AC-2 [auto: TEST-0009] Names unique aboard.
- [x] AC-3 [auto: TEST-0010] One captain, one quartermaster.
- [x] AC-4 [auto: TEST-0011] A new captain replaces the old one.
- [x] AC-5 [auto: TEST-0012] Status changes follow the articles.
- [x] AC-6 [auto: TEST-0013] The captain cannot leave.
- [x] AC-7 [auto: TEST-0014] Changes are recorded.
- [x] AC-8 [auto: TEST-0015] Shares read well for humans.
- [x] AC-9 [doc] `crew` module and changelog updated.

## Closure
- **Done:** The `crew` package and its migration. The module gained `name_taken`, `not_aboard` and `captain_required`.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0008 to TEST-0015.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
