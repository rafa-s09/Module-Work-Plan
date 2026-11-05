---
kind: task
id: T-0001
status: done
size: M
created: 2026-11-05
modules: [db]
reads: []
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0001 — Lay the keel: database and migrations

## Goal
Arrr, no ship floats without a keel. Give the ledger its SQLite file, migrations and transactions, so every other module has somewhere to keep its treasure.

## Scope
- Includes: the `backend/` skeleton, `connect`, `transaction`, the migration runner.
- Excludes: any table of another module.

## Acceptance criteria
- [x] AC-1 [auto: TEST-0001] Migrations apply in order, once.
- [x] AC-2 [auto: TEST-0002] Foreign keys are enforced.
- [x] AC-3 [auto: TEST-0003] A failing migration leaves nothing behind.
- [x] AC-4 [auto: TEST-0004] Transactions roll back, including joined ones.
- [x] AC-5 [auto: TEST-0005] The file comes from `LEDGER_DB`.
- [x] AC-6 [auto: TEST-0006] Migrations take under 1 second.
- [x] AC-7 [auto: TEST-0007] Times are UTC text.
- [x] AC-8 [doc] `db` module and changelog updated.

## Closure
- **Done:** The `db` package and the runner.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0001 to TEST-0007.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
