# Constitution

> Rules every agent follows in this project. Replace the `<placeholders>` with this project's rules.

## 1. Reading protocol
1. Read this constitution and the current task.
2. Read only the modules in the task's `modules:` and `reads:`, the SPECs in `specs:` and the ADRs in `decisions:`.
3. Read a module's durable SPECs or ADRs, `PROJECT.md` or `glossary.md` only if the task touches what they cover or a domain question arises.
4. Use `map.md` only to find a module not listed in the task.
5. Resources (icons, images, design files, documents) live in `content/`. Open a resource only when the task cites it or needs it; list the folder to find one, never load it all.
6. Never read the whole `modules/` folder, `tasks/done/` (except tasks in `refs:`), SPECs that are `incorporated` or `archived`, or ADRs that are `rejected`, `superseded` or `deprecated`, unless the task cites them.
7. If you need a module outside the manifest, record it in the closure.

## 2. Allowed / must use
- Approved languages and libraries: <...>
- Test framework: <...> — command: `<...>`
- Skills: <mwp, ...>
- Code style: <... or "follow the configured linter">

## 3. Not allowed
- Adding a dependency without recording and justifying it in the task.
- Writing modules not listed in `modules:`.
- Editing a task in `tasks/done/`.
- Working on a task another agent holds, or writing a module locked by another task (`mwp locks`).
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
- Size M and up: write the test first, confirm it fails, then implement.
- Rule of 3: if a test or criterion still fails after 3 fix attempts, **stop**. Report what you tried and your hypothesis, and wait for a person to decide whether to go on or take another route. Rerunning the regression suite does not count.
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

**Replanning.** When the plan cannot reach the goal and a person agrees to replan: mark each criterion not met `- [-] AC-N ... — failed: <reason>`, fill `## Closure` with `**Replanning:**` (why the plan failed and the new tasks), and close with `mwp task close T-XXXX --replanned` (`status: replanned`, moved to `tasks/done/`). The new tasks cite it in `refs:`.

## 8. Conflicts — priority order
> The stop rules here and in §5 are brakes, not punishments: they prevent excess and work outside the scope. Following them saves effort for the agent and for the people working with it.

1. `constitution.md`
2. Accepted ADRs
3. Module (including durable SPECs and registered TESTs)
4. Task

- Task contradicts a module, durable SPEC or ADR: **stop** and ask.
- Code contradicts the module: **stop**, describe the divergence and ask which one is right.
- An existing TEST fails and seems to contradict the task: **stop** and ask.
- Content the task needs cannot be found (a file, a resource in `mwp/content/`, a document it cites): **stop** and say what is missing. Never invent or download a replacement.
- A link or reference is broken (a path, a URL, an ID that `mwp context` reports as `MISSING`): **stop** and ask. A module's `code:` or `tests:` path that the task is about to create is not broken.
- An unexpected error that cannot be worked around inside the task's scope (a command or tool fails, a dependency or permission is missing): **stop**, report the error and what was tried, and release the task with `mwp task release T-XXXX --status blocked`.
- Never resolve a conflict silently.

## 9. When in doubt
- Ambiguity that changes the result: ask.
- Trivial detail (variable name, import order): decide and move on.

## 10. Task sizes
Size by the work **and** by the effort to verify it: many `[manual]` criteria or a long verification raise the size.
Only a person sets `plan_approved:`; an agent never writes it. `mwp task close` refuses an L, XL or XXL task without it.

| Size | When | Rule |
|---|---|---|
| XS | A trivial change, like bumping a version | Short template; no test first |
| S | A few lines, one module | Short template |
| M | A feature in one or two modules | Test first |
| L | Several modules | Test first; `## Plan` approved by a person (`plan_approved:`) before implementing |
| XL | Several modules, or many `[manual]` criteria, or a long verification | As L, and the plan goes into a transient SPEC |
| XXL | Too big for one task, or far outside the project's field | The plan is the list of smaller tasks; a person approves it, and the XXL task is not implemented directly |
