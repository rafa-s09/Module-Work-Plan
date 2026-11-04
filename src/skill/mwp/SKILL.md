---
name: mwp
description: Work in a project documented with MWP (Module Work Plan). Use when the project has a mwp/ folder, or when asked to execute, create, plan or close a task (T-XXXX), document a module, write a SPEC (SP-XXXX) or an ADR. Explains how to load only the context a task needs, how to use the `mwp` CLI, and how to finish a task with evidence.
---

# MWP — Module Work Plan

> Draft. If the `mwp` CLI is not installed, use the manual equivalent listed for each command.

`mwp/constitution.md` is the source of truth for the project's rules. This skill explains **how** to work with the format; if it disagrees with the constitution, the constitution wins.

## Files

| File | Role |
|---|---|
| `mwp/constitution.md` | Rules: what to read, what is allowed, when to stop |
| `mwp/tasks/open/`, `mwp/tasks/done/` | Tasks: what changes. Done tasks are never edited |
| `mwp/modules/` | Modules: how the system is today |
| `mwp/specs/` | SPECs: specifications too big for a module or task |
| `mwp/decisions/` | ADRs: why a choice was made |
| `mwp/map.md` | Generated index of modules, one line each |
| `mwp/PROJECT.md`, `mwp/glossary.md` | Overview, commands, domain terms |
| `mwp/content/` | Optional. Resources for the project: icons, images, design files, documents |
| `.mwp/ledger.json` | Last reserved ID per prefix; written by the CLI, never by hand |

Files starting with `_` are templates: copy them, never treat them as real tasks, modules, SPECs or ADRs. `mwp/modules/README.md` explains the folder and is not a module.

Every file starts with YAML front matter. Read the front matter first; it tells you what else to load.

## IDs

| ID | What | Scope |
|---|---|---|
| `T-0043` | Task | Project |
| `SP-0003` | SPEC | Project |
| `ADR-0002` | Decision | Project |
| `TEST-0007` | Test | Project |
| `FR-01` / `NFR-01` | Functional / non-functional rule | Module, cited as `order#FR-01` |
| `AC-1` | Acceptance criterion | Task |

- Project IDs are global, sequential, at least 4 digits, never reused.
- Get a new one with `mwp next <T|SP|ADR|TEST> --reserve`. `--reserve` records it in the ledger so nobody else gets it, even if the file is later deleted.
- Always search with a word boundary: `\bT-0043\b`. `TEST-0043` contains `T-0043`, `NFR-01` contains `FR-01`.
- Company work codes (Jira, ticket) live in the task's `external:` field. `mwp context` accepts the external code in place of the task ID, and `mwp task-list` shows it.
- Refer to files by ID, never by path.

## Executing a task

When asked "execute T-0043" (or a company code that maps to it):

1. **Load context.** Run `mwp context T-0043 --paths` and read exactly those files. Load more only when the constitution allows it, and stop as soon as you have enough. Never read the whole `modules/` folder or `tasks/done/`. If it prints `MISSING`, the task cites something that does not exist: stop and ask.
2. **Set `status: in_progress`** and create the branch `task/T-0043-slug`.
3. **Size L:** write the `## Plan` and **stop** until a person approves it.
4. **Size M and L:** write the tests first (`mwp next TEST --reserve` for each new ID), run them, confirm they fail for the expected reason. Test names start with their ID: `TEST-0007: ...`.
5. **Implement**, writing only code that belongs to modules in `modules:`.
6. **Verify** every acceptance criterion:
   - `[auto: TEST-XXXX]`: the test passes.
   - `[manual]`: follow the criterion's `Verify:` steps.
   - `[regression]`: all tests of the touched modules pass.
   - `[doc]`: the module and its changelog reflect the change.
7. **Update the modules:** new rules, the `## Tests` table, `updated:`, the changelog row with the task ID. Then run `mwp remap`: it checks module paths, dependencies and registered tests, and rewrites `map.md`.
8. **Close:** fill `## Closure` with evidence for every criterion (command and result, or what you observed), set `status: done`, move the file to `tasks/done/`, and run `mwp lint`. The task is not done until lint reports no errors.

If a `[manual]` criterion could not be checked, set `status: review`, list what is missing and stop.

## Creating a task

1. `mwp task new "<title>" --size M --modules order [--reads customer] [--refs T-0042] [--external PROJ-1234]` creates `mwp/tasks/open/T-XXXX-slug.md` with the next ID reserved and today's date. Size S uses the short template.
2. Fill Goal, Scope and Acceptance criteria. `modules:` lists every module the task may write; a task without it is invalid.
3. Every criterion says how it is verified. Tasks that change code have at least one `[regression]` and one `[doc]`. Vague criteria ("works well", "clean code") are not allowed.
4. Run `mwp context T-XXXX`. If the token total is large, split the task.

To see what is pending: `mwp task-list` (or `mwp task list`): open tasks, `--status <s>`, `--module <id>`, `--all`.

## Recording a decision

- `mwp adr new "<title>" --modules order` creates `ADR-XXXX-slug.md` as `proposed`. Fill in Context, Decision and Consequences.
- Replacing a decision: `mwp adr new "<title>" --supersedes ADR-0002`. The old ADR is set to `superseded` with `superseded_by:`. Both changes belong to the same task.
- Only a person sets `accepted`. Never edit the body of an accepted ADR; factual fixes go in `## Amendments`.
- `mwp adr list [--status accepted]` shows the decisions in force.

## Using resources (`mwp/content/`)

`mwp/content/` is the default place for resources: icons, images, Figma files or exports, PDFs, reference documents, sample data. The folder is optional; the CLI ignores it.

- **Need a resource?** Look in `mwp/content/` first. List the folder to find it; never open everything.
- **Open only what the task needs.** Binary files (images, `.fig`, PDFs) are referenced by path; open one only if you must inspect it.
- **Tasks cite resources by path** (`mwp/content/icons/cart.svg`) in Scope or in a criterion. Resources have no ID.
- **The app needs it at build or run time?** Copy it into the code's asset folder; `content/` keeps the reference copy.
- **New resource from the task?** Save it in `mwp/content/`, in a subfolder by type (`icons/`, `images/`, `design/`, `docs/`), and list it in the closure.
- **Not there?** Ask. Never invent or download a replacement.

## Documenting modules

To create modules for code that has none, or to check existing modules against the code and the ADRs, use the `module` skill.

## Rules that are easy to break

- **Existing tests are rules.** Never change a test to make it pass. Only tests in `modifies_tests:` may change.
- **Born in the task, lives in the module.** Anything still true after the task closes goes into the module before closing.
- **Rework is a new task** with `refs: [T-0043]`. Never edit a done task.
- **Never edit the table in `map.md` by hand.** It is generated from module front matter.
- **Module outside the manifest:** read it if you must, and record it in the closure.

## Stop and ask when

- The task contradicts a module, a durable SPEC or an accepted ADR.
- The code contradicts its module.
- An existing test fails and seems to contradict the task.
- `mwp context` reports `MISSING`, or `mwp lint` reports an error you cannot fix inside the task's modules.
- An ambiguity would change the result.

Priority on conflict: constitution > accepted ADRs > module > task. Never resolve a conflict silently.

## CLI reference

| Command | What it does | Without the CLI |
|---|---|---|
| `mwp init [dir] [--name <n>] [--no-skill]` | Creates `mwp/`, `AGENTS.md`, `CLAUDE.md` and this skill; never overwrites | Copy the templates |
| `mwp lint [--strict]` | Checks front matter, IDs, references, closures, registered tests, map, size limits. Exit 1 on errors (`--strict`: also on warnings) | Check by reading the files |
| `mwp map [--write \| --check]` | Prints the module table; `--write` updates `map.md`; `--check` fails if it is stale | Rebuild the table between the markers from module front matter |
| `mwp remap [--dry-run]` | Rechecks modules (paths, `depends_on`, registered tests) and updates `map.md` | Same checks by hand, then rebuild the table |
| `mwp task new <title> [--size S\|M\|L] [--modules a,b] [--reads a,b] [--refs T-XXXX] [--external CODE]` | Creates an open task from the template with the next T- ID reserved | `mwp next T --reserve` and copy the task template |
| `mwp task-list [--status <s>] [--module <id>] [--all]` | Lists tasks with status, size, title and external codes | List `mwp/tasks/open/` |
| `mwp next <T\|SP\|ADR\|TEST> [--reserve]` | Next free ID from MWP docs, module test folders and the ledger | Highest existing ID with that prefix, plus one |
| `mwp context <T-XXXX \| code> [--paths]` | Files the task loads, with a token estimate; `MISSING` for broken references | Constitution + task + files in its front matter |
| `mwp adr new <title> [--modules a,b] [--supersedes ADR-XXXX]` / `mwp adr list [--status <s>]` | Creates a proposed ADR / lists ADRs | Copy `decisions/_template.md` |
