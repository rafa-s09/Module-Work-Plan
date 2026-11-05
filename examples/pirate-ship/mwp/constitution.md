# Constitution

> The articles every agent signs before touching this ship. Break one and ye walk the plank. Or, worse, ye get a failing lint.

## 0. Tone
- Prose is written in pirate English: module descriptions, pitfalls, comments, UI messages, error messages, commit bodies.
- **Rules, acceptance criteria, numbers, IDs, field names, paths and code stay exact.** A pirate may drink, but the ledger may not.
- A criterion like *"the loot is split more or less fairly"* is not allowed, no matter how much rum is involved.
- Code identifiers are plain English (`split_plunder`, not `divvy_the_booty`), so any sailor can read them.

## 1. Reading protocol
1. Read this constitution and the current task.
2. Read only the modules in the task's `modules:` and `reads:`, the SPECs in `specs:` and the ADRs in `decisions:`.
3. Read a module's durable SPECs or ADRs, `PROJECT.md` or `glossary.md` only if the task touches what they cover or a domain question arises.
4. Use `map.md` only to find a module not listed in the task.
5. Resources (icons, images, design files, documents) live in `content/`. Open a resource only when the task cites it or needs it; list the folder to find one, never load it all.
6. Never read the whole `modules/` folder, `tasks/done/` (except tasks in `refs:`), SPECs that are `incorporated` or `archived`, or ADRs that are `rejected`, `superseded` or `deprecated`, unless the task cites them.
7. If you need a module outside the manifest, record it in the closure.

## 2. Allowed / must use
- Back: Python 3.12+, FastAPI, Pydantic (comes with FastAPI), the standard `sqlite3` module, `hashlib.scrypt` and `secrets` for passwords and tokens.
- Front: Angular (standalone components, signals), TypeScript strict mode, the Angular `HttpClient`. No UI component library; the theme is our own (see `deck`).
- Database: SQLite through `sqlite3`. No ORM.
- Test frameworks: pytest — `cd backend && pytest`; Angular test runner — `cd frontend && npm test`.
- Skills: `mwp` (workflow), `module` (create or audit modules), `ponytail` (full level, on every task that writes code).
- Code style: `ruff` for Python, the Angular CLI defaults for TypeScript.
- Money: whole reales in an `int` (Python) or `number` holding an integer (TypeScript). Never a float, never a decimal, never doubloons in storage. See `loot`.

### Working in ponytail mode
- The smallest change that meets every acceptance criterion. No feature, option or abstraction the task does not ask for.
- Standard library and what is already in the code before anything new.
- A known limit gets a comment: `shortcut: <the limit>, <when to upgrade>`.
- Ponytail never cuts: validation at the API boundary, password and token handling, money correctness, accessibility, or a test the task requires.
- If ponytail and this constitution disagree, this constitution wins.

## 3. Not allowed
- Adding a dependency without recording and justifying it in the task.
- Writing modules not listed in `modules:`.
- Editing a task in `tasks/done/`.
- Editing the body of an accepted ADR, except `## Amendments` entries for factual corrections.
- Changing or removing an existing TEST unless the task lists it in `modifies_tests:`.
- Checking a criterion without evidence.
- Deleting files without an explicit request.
- Deleting a crew member, a voyage or a split from the database. They are kept forever (see `db#FR-05`).
- Storing or logging a password, a password hash or a session token in plain sight (logs, errors, API responses).
- Business rules in the deck. The deck shows and asks; the back decides.
- Pushing, merging or deploying.

## 4. Versioning (git)
- Branch: `task/T-XXXX-slug`
- Commit: `T-XXXX: short imperative description`
- One commit per logical step; the last commit includes the module updates.

## 5. Tests
- Domain rules, contracts, bug fixes and measurable NFRs require a TEST.
- The test name in code starts with its ID: `TEST-0007: ...` (pytest: in the docstring or the test id; Angular: in the `it(...)` title).
- Size M and L tasks: write the test first, confirm it fails, then implement.
- Every new TEST is registered in the module's `## Tests` table (ID, covered rule, scenario, file).
- Project IDs (`T`, `SP`, `ADR`, `TEST`) are global, sequential, at least 4 digits (`T-0043`) and never reused: use `mwp next <prefix> --reserve`, or the highest one in the project plus one.
- Company work codes go in the task's `external:` field, never in `id:`.
- Search IDs with a word boundary (`\bT-0043\b`): `TEST-0043` contains `T-0043`, `NFR-01` contains `FR-01`.
- A bug fix always adds a TEST that reproduces the bug.
- Back tests use a fresh in-memory database per test (see `db#FR-06`). No test touches `backend/data/`.

## 6. SPECs and ADRs
- A durable SPEC extends its module and follows the same truth rules as the module.
- A transient SPEC: at closure, move durable parts into the module and set the SPEC to `incorporated` or `archived`.
- A decision the task needs is written as a `proposed` ADR. Only a person sets it to `accepted`.
- Replacing a decision: the new ADR lists the old one in `supersedes:`; the old one gets `status: superseded` and `superseded_by:`, in the same task.

## 7. Definition of done
A task moves to `tasks/done/` only when:
- [ ] Every acceptance criterion is met, with evidence in the closure.
- [ ] Regression of the modules in `modules:` passes.
- [ ] Every affected module is updated: rules, `## Tests` table, `code:` and `tests:` paths, `updated:` and changelog.
- [ ] The task's transient SPECs are incorporated or archived.
- [ ] `## Closure` is filled in, including what ponytail skipped.
- [ ] `mwp lint` finishes without errors.
- [ ] `status: done` in front matter and the file is in `tasks/done/`.

If a `[manual]` criterion could not be verified: `status: review`, list what is missing and stop.

## 8. Conflicts — priority order
1. `constitution.md`
2. Accepted ADRs
3. Module (including durable SPECs and registered TESTs)
4. Task

- Task contradicts a module, durable SPEC or ADR: **stop** and ask.
- Code contradicts the module: **stop**, describe the divergence and ask which one is right.
- An existing TEST fails and seems to contradict the task: **stop** and ask.
- Never resolve a conflict silently. A quiet mutiny is still a mutiny.

## 9. When in doubt
- Ambiguity that changes the result: ask.
- Trivial detail (variable name, import order): decide and move on.
