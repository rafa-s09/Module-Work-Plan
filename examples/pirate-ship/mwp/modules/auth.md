---
kind: module
id: auth
type: domain
summary: The gangplank. Username and password login, sessions of two watches, and who may give which orders.
code: [backend/app/auth, backend/migrations/0002_auth.sql]
tests: [backend/tests/auth]
specs: []
depends_on: [crew, db]
updated: 2026-11-05
---

# auth — The Gangplank

No one boards this ledger without the password, matey. Every account belongs to one member of the crew, and what ye may do depends on yer rank. Fail the password too often and the gangplank is pulled up for a while.

## Functional rules
- **FR-01** An account has a `username` (3 to 32 characters, letters, digits, `_` or `-`, unique ignoring case), a password hash and exactly one crew member. A crew member has at most one account. Tests: TEST-0016
- **FR-02** Passwords have at least 10 characters. They are stored only as a `hashlib.scrypt` hash (n=2^14, r=8, p=1) with a 16-byte random salt per account. Tests: TEST-0017
- **FR-03** Login takes a username and a password. On success it creates a session with a token of 32 random bytes (`secrets.token_urlsafe(32)`) and returns it. Only the SHA-256 of the token is stored. Tests: TEST-0018
- **FR-04** A wrong username and a wrong password give the same answer: `bad_credentials`. The answer never says which one was wrong. Tests: TEST-0019
- **FR-05** After 5 failed logins in a row for one username, real or not, that username is locked for 15 minutes. A login during the lock answers `locked` even with the right password. A successful login resets the count. Tests: TEST-0020, TEST-0021
- **FR-06** A session lasts 8 hours (two watches) from login and is not extended by use. Logout ends it at once. An expired or ended token answers `not_logged_in`. Tests: TEST-0022
- **FR-07** The first captain's account is created only from the command line: `python -m app.auth.make_captain`. It refuses if any account already exists. Tests: TEST-0023
- **FR-08** Only a member whose status is `aboard` can log in; anyone else gets `bad_credentials`. A member who goes ashore, is marooned or goes to Davy Jones loses every open session at once, and coming back does not revive them. Tests: TEST-0024, TEST-0057
- **FR-09** Orders by rank, enforced by the API with `require`. Anything not listed here is allowed to any logged-in member. Tests: TEST-0025, TEST-0055

| Order | Code | Allowed to |
|---|---|---|
| Create an account, reset another member's password | `manage_accounts` | captain |
| Add a member, change a rank, change a status | `manage_crew` | captain |
| Create a voyage, change the voyage state | `move_ship` | captain |
| Record plunder and compensation, run the split | `manage_loot` | quartermaster |
| Change the hold | `change_hold` | quartermaster, boatswain |

- **FR-10** A member can change their own password by giving the current one (`wrong_password` otherwise). All their other sessions end. Tests: TEST-0026
- **FR-11** A reset replaces the password and ends every session of that member. Tests: TEST-0027

## Non-functional rules
- **NFR-01** Checking a password takes under 500 ms on the test machine, for a right or a wrong password. Tests: TEST-0028

## Contracts / interfaces
- `login(conn, username, password) -> token`, `logout(conn, token)`.
- `current_member(conn, token) -> row` — raises `not_logged_in`.
- `require(member, order)` — raises `not_allowed` when FR-09 forbids it. `allowed_orders(rank) -> list[str]`.
- `create_account`, `create_first_captain`, `change_own_password`, `reset_password`, `end_sessions`.
- `change_status(conn, id, status, by)` — `crew.change_status` plus FR-08.
- Error codes: `bad_credentials`, `locked`, `not_logged_in`, `not_allowed`, `weak_password`, `wrong_password`, `username_taken`, `already_has_account`, `accounts_exist`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0016 | FR-01 | Username rules, uniqueness, one account per member | backend/tests/auth/test_auth.py |
| TEST-0017 | FR-02 | Short passwords refused; hashed, salted apart | backend/tests/auth/test_auth.py |
| TEST-0018 | FR-03 | Token returned, only its hash stored | backend/tests/auth/test_auth.py |
| TEST-0019 | FR-04 | Same answer, and the hash is spent either way | backend/tests/auth/test_auth.py |
| TEST-0020 | FR-05 | Lock after 5, real or unknown username; lock expires | backend/tests/auth/test_auth.py |
| TEST-0021 | FR-05 | Success resets the count | backend/tests/auth/test_auth.py |
| TEST-0022 | FR-06 | 8 hours, not extended; logout and expiry end it | backend/tests/auth/test_auth.py |
| TEST-0023 | FR-07 | First captain only once | backend/tests/auth/test_auth.py |
| TEST-0024 | FR-08 | Leaving ends sessions for good; only aboard logs in | backend/tests/auth/test_auth.py |
| TEST-0025 | FR-09 | Orders of each rank | backend/tests/auth/test_auth.py |
| TEST-0026 | FR-10 | Own password needs the current one, ends other sessions | backend/tests/auth/test_auth.py |
| TEST-0027 | FR-11 | Reset ends every session | backend/tests/auth/test_auth.py |
| TEST-0028 | NFR-01 | Check under 500 ms | backend/tests/auth/test_auth.py |

## Related decisions
- None yet.

## Known pitfalls
- Compare password hashes with `hmac.compare_digest`. Tokens need no such care: they are looked up by their hash.
- A login for a username that does not exist must still run scrypt, or a spy can time which names are real. Failures are kept per typed username for the same reason.
- Never log the request body of the login endpoint.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0003 | Built. The lock covers unknown usernames too; added FR-11; NFR-01 is now only an upper limit, since a lower one fails on fast machines. |
