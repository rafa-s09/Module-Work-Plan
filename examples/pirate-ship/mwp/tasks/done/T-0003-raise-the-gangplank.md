---
kind: task
id: T-0003
status: done
size: M
created: 2026-11-05
modules: [auth]
reads: [crew, db]
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0003 — Raise the gangplank: login and orders

## Goal
No stranger boards. Build accounts with hashed passwords, sessions, the lock after bad tries, and the table of who may give which order.

## Scope
- Includes: accounts, login, sessions, lock, orders by rank, `make_captain`, migration `0002`.
- Excludes: the HTTP layer (T-0006) and the login screen (T-0007).

## Acceptance criteria
- [x] AC-1 [auto: TEST-0016] Account rules.
- [x] AC-2 [auto: TEST-0017] Passwords are hashed and salted.
- [x] AC-3 [auto: TEST-0018] Only the token's hash is stored.
- [x] AC-4 [auto: TEST-0019] Same answer for a bad name and a bad password.
- [x] AC-5 [auto: TEST-0020, TEST-0021] Lock after 5 failures.
- [x] AC-6 [auto: TEST-0022] Sessions last 8 hours.
- [x] AC-7 [auto: TEST-0023] First captain only once.
- [x] AC-8 [auto: TEST-0024] Leaving the ship ends sessions.
- [x] AC-9 [auto: TEST-0025] Orders by rank.
- [x] AC-10 [auto: TEST-0026, TEST-0027] Changing and resetting passwords.
- [x] AC-11 [auto: TEST-0028] A password check takes under 500 ms.
- [x] AC-12 [doc] `auth` module and changelog updated.

## Closure
- **Done:** The `auth` package and its migration. The lock now covers unknown usernames, and NFR-01 became an upper limit only.
- **Acceptance evidence:** `cd backend && pytest`: 104 passed on 2026-11-05 (Python 3.15). `ruff check .`: all checks passed.
- **TESTs created:** TEST-0016 to TEST-0028.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** None.
