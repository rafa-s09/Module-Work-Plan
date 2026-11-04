# Constitution

> Rules every agent follows in this project. Replace the `<placeholders>` with this project's rules.

## 1. Reading protocol
1. Read this constitution and the current task.
2. Read only the modules in the task's `modules:` and `reads:`, the SPECs in `specs:` and the ADRs in `decisions:`.
3. Read a module's durable SPECs or ADRs, `PROJECT.md` or `glossary.md` only if the task touches what they cover or a domain question arises.
4. Use `map.md` only to find a module not listed in the task.
5. Never read the whole `modules/` folder, `tasks/done/` (except tasks in `refs:`), SPECs that are `incorporated` or `archived`, or ADRs that are `rejected`, `superseded` or `deprecated`, unless the task cites them.
6. If you need a module outside the manifest, record it in the closure.

## 2. Allowed / must use
- Approved languages and libraries: <...>
- Test framework: <...> — command: `<...>`
- Skills: <mwp, ...>
- Code style: <... or "follow the configured linter">

## 3. Not allowed
- Adding a dependency without recording and justifying it in the task.
- Writing modules not listed in `modules:`.
- Editing a task in `tasks/done/`.
- Editing the body of an accepted ADR, except `## Amendments` entries for factual corrections.
- Changing or removing an existing TEST unless the task lists it in `modifies_tests:`.
- Checking a criterion without evidence.
- Deleting files without an explicit request.
- Pushing, merging or deploying <unless the task says so>.

## 4. Versioning (git)
- Branch: `task/T-XXXX-slug`
- Commit: `T-XXXX: short imperative description`
- One commit per logical step; the last commit includes the module updates.

## 5. Tests
- Domain rules, contracts, bug fixes and measurable NFRs require a TEST.
- The test name in code starts with its ID: `TEST-0007: ...`.
- Size M and L tasks: write the test first, confirm it fails, then implement.
- Every new TEST is registered in the module's `## Tests` table (ID, covered rule, scenario, file).
- Project IDs (`T`, `SP`, `ADR`, `TEST`) are global, sequential, at least 4 digits (`T-0043`) and never reused: use `mwp next <prefix> --reserve`, or the highest one in the project plus one.
- Company work codes go in the task's `external:` field, never in `id:`.
- Search IDs with a word boundary (`\bT-0043\b`): `TEST-0043` contains `T-0043`, `NFR-01` contains `FR-01`.
- A bug fix always adds a TEST that reproduces the bug.

## 6. SPECs and ADRs
- A durable SPEC extends its module and follows the same truth rules as the module.
- A transient SPEC: at closure, move durable parts into the module and set the SPEC to `incorporated` or `archived`.
- A decision the task needs is written as a `proposed` ADR. Only a person sets it to `accepted`.
- Replacing a decision: the new ADR lists the old one in `supersedes:`; the old one gets `status: superseded` and `superseded_by:`, in the same task.

## 7. Definition of done
A task moves to `tasks/done/` only when:
- [ ] Every acceptance criterion is met, with evidence in the closure.
- [ ] Regression of the modules in `modules:` passes.
- [ ] Every affected module is updated: rules, `## Tests` table, `updated:` and changelog.
- [ ] The task's transient SPECs are incorporated or archived.
- [ ] `## Closure` is filled in.
- [ ] `mwp lint` finishes without errors (when the CLI is installed).
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
- Never resolve a conflict silently.

## 9. When in doubt
- Ambiguity that changes the result: ask.
- Trivial detail (variable name, import order): decide and move on.
