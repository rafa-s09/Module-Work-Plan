---
kind: task
id: T-0005
status: done
size: M
created: 2026-11-05
modules: [loot]
reads: [voyage, crew, db]
specs: []
refs: []
decisions: [ADR-0001]
modifies_tests: []
external: []
---

# T-0005 — Divide the plunder by shares

## Goal
The heart of the ledger. Pay the wounded first, divide the rest by shares in whole reales, and send what won't divide to the chest, without losing a single real.

## Scope
- Includes: plunder, compensation, the split, the chest, money display, migration `0005`.
- Excludes: screens (T-0007).

## Acceptance criteria
- [x] AC-1 [auto: TEST-0039] Money display.
- [x] AC-2 [auto: TEST-0040] Plunder only while plundering.
- [x] AC-3 [auto: TEST-0041, TEST-0042] Compensation.
- [x] AC-4 [auto: TEST-0043] The split, to the real.
- [x] AC-5 [auto: TEST-0044, TEST-0048] Refusals.
- [x] AC-6 [auto: TEST-0045] Once only, at the right time.
- [x] AC-7 [auto: TEST-0046] Dead and marooned pay the chest.
- [x] AC-8 [auto: TEST-0047] No real is lost.
- [x] AC-9 [doc] `loot` module and changelog updated.

## Closure
- **Done:** The `loot` package and its migration. FR-06 and FR-07 were made exact with `paid_to`: their first wording counted a dead member's money twice.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0039 to TEST-0048.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
