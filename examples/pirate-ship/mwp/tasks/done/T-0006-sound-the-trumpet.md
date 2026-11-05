---
kind: task
id: T-0006
status: done
size: M
created: 2026-11-05
modules: [api]
reads: [auth, crew, voyage, hold, loot]
specs: []
refs: []
decisions: [ADR-0001]
modifies_tests: []
external: []
---

# T-0006 — Sound the trumpet: the HTTP API

## Goal
Carry orders from the deck to the ship. Endpoints over the domain, one shape for every error, and the rank's orders enforced on every guarded route.

## Scope
- Includes: every endpoint, error handling, `errors.py`, `main.py`.
- Excludes: screens (T-0007) and deployment.

## Acceptance criteria
- [x] AC-1 [auto: TEST-0049] Login and `me`.
- [x] AC-2 [auto: TEST-0050] Token required.
- [x] AC-3 [auto: TEST-0051, TEST-0052, TEST-0053] Status codes and error shape.
- [x] AC-4 [auto: TEST-0054] A 500 hides the details.
- [x] AC-5 [auto: TEST-0055] Orders enforced.
- [x] AC-6 [auto: TEST-0056] A whole voyage through the API.
- [x] AC-7 [auto: TEST-0057] A status change ends sessions.
- [x] AC-8 [auto: TEST-0058] GETs under 200 ms.
- [x] AC-9 [doc] `api` module and changelog updated.

## Closure
- **Done:** The `api` package and `main.py`. Added the endpoints the deck needs; dropped CORS in favor of the dev proxy.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0049 to TEST-0058.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
