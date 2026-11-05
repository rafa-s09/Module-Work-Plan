---
kind: module
id: auth
type: domain
summary: The gangplank. Username and password login, sessions of two watches, and who may give which orders.
code: []
tests: []
specs: []
depends_on: [crew, db]
updated: 2026-11-04
---

# auth — The Gangplank

No one boards this ledger without the password, matey. Every account belongs to one member of the crew, and what ye may do depends on yer rank. Fail the password too often and the gangplank is pulled up for a while.

> Planned code: `backend/app/auth/`. Planned tests: `backend/tests/auth/`.

## Functional rules
- **FR-01** An account has a `username` (3 to 32 characters, letters, digits, `_` or `-`, unique ignoring case), a password hash and exactly one crew member. A crew member has at most one account.
- **FR-02** Passwords have at least 10 characters. They are stored only as a `hashlib.scrypt` hash (n=2^14, r=8, p=1) with a 16-byte random salt per account.
- **FR-03** Login takes a username and a password. On success it creates a session with a token of 32 random bytes (`secrets.token_urlsafe(32)`) and returns it. Only the SHA-256 of the token is stored.
- **FR-04** A wrong username and a wrong password give the same answer: `bad_credentials`. The answer never says which one was wrong.
- **FR-05** After 5 failed logins in a row for one username, that username is locked for 15 minutes. A login during the lock answers `locked` even with the right password. A successful login resets the count.
- **FR-06** A session lasts 8 hours (two watches) from login and is not extended by use. Logout ends it at once. An expired or ended token answers `not_logged_in`.
- **FR-07** The first captain's account is created only from the command line: `python -m app.auth.make_captain`. It refuses if any account already exists.
- **FR-08** Only a member whose status is `aboard` can log in. A member who goes ashore, is marooned or goes to Davy Jones loses every open session at once (see crew#FR-05).
- **FR-09** Orders by rank. Anything not listed here is allowed to any logged-in member:

| Order | Allowed to |
|---|---|
| Create an account, reset another member's password | captain |
| Add a member, change a rank, change a status | captain |
| Change the voyage state | captain |
| Record plunder and compensation, run the split | quartermaster |
| Change the hold | quartermaster, boatswain |

- **FR-10** A member can change their own password by giving the current one. All their other sessions end.

## Non-functional rules
- **NFR-01** Checking a password takes between 50 ms and 500 ms on the test machine, for a right or a wrong password.

## Contracts / interfaces
- `login(username, password) -> token` / `logout(token)`.
- `current_member(token) -> CrewMember` — raises `not_logged_in`.
- `require(member, order)` — raises `not_allowed` when FR-09 forbids it.
- Error codes: `bad_credentials`, `locked`, `not_logged_in`, `not_allowed`, `weak_password`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- Comparing token hashes with `==` leaks timing. Use `hmac.compare_digest`.
- A login for a username that does not exist must still run scrypt, or NFR-01 and FR-04 tell a spy which names are real.
- Never log the request body of the login endpoint.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
