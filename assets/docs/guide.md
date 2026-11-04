# Using MWP: skill and CLI

This guide covers the day-to-day use of MWP: installing the `mwp` CLI, setting up a project, the workflow with an agent, and every command. For the ideas behind the format, start with the [README](../../README.md).

- [1. Install](#1-install)
- [2. Set up a project](#2-set-up-a-project)
- [3. Concepts in one page](#3-concepts-in-one-page)
- [4. The workflow](#4-the-workflow)
- [5. The agent skills](#5-the-agent-skills)
- [6. CLI reference](#6-cli-reference)
- [7. Continuous integration](#7-continuous-integration)
- [8. Troubleshooting](#8-troubleshooting)

## 1. Install

The CLI is written in Rust (1.85+) and has no dependencies. Install [Rust](https://rustup.rs), then run the installer from a checkout of this repository:

**Linux/macOS**
```bash
./src/install.sh
```

**Windows (Powershell)**
```powershell
.\src\install.ps1
```
If PowerShell blocks scripts: `powershell -ExecutionPolicy Bypass -File .\src\install.ps1`.

Pass a project folder to also run `mwp init` there (`./src/install.sh ../store`). Both scripts run `cargo install --path src/client --locked --force` and say if the install folder is missing from your `PATH`. Check with:

```bash
mwp --version
```

The templates and the skills are embedded in the binary at build time, so run the installer again after changing `templates/` or `src/skill/`.

The CLI is optional. Every command has a manual equivalent, listed in the [mwp skill](../../src/skill/mwp/SKILL.md#cli-reference).

## 2. Set up a project

```bash
cd your-project
mwp init --name "Store"
```

`init` creates the files below and **never overwrites** an existing file. It prints `skip` for files that are already there.

```
AGENTS.md                       # pointer to the constitution, for any agent
CLAUDE.md                       # same pointer, for Claude Code
.claude/skills/mwp/SKILL.md     # workflow skill (both skills omitted with --no-skill)
.claude/skills/module/SKILL.md  # module skill
mwp/
├── PROJECT.md
├── constitution.md
├── map.md
├── glossary.md
├── modules/README.md, _template.md
├── tasks/open/.gitkeep, _template-task.md, _template-task-s.md
├── tasks/done/.gitkeep
├── specs/_template.md
├── decisions/_template.md
└── content/.gitkeep
```

Then:

1. **Fill `mwp/PROJECT.md`**: what the project is, the stack, and the commands to run and test it.
2. **Adapt `mwp/constitution.md`**: replace each `<placeholder>` with your project's rules (libraries, test command, what the agent may not do).
3. **Add to `.gitignore`**: the ledger is versioned, the rest of `.mwp/` is local.

   ```gitignore
   .mwp/*
   !.mwp/ledger.json
   ```

4. **Document the first module**: the one you will change first, not the whole system (see [4.1](#41-document-a-module)). For an existing codebase, the `module` skill can propose and write the modules for you.

## 3. Concepts in one page

| File | What it holds | Changes |
|---|---|---|
| Module | How a part of the system works **today**: rules, tests, contracts, pitfalls | Updated with every change |
| Task | What **changes**: goal, scope, acceptance criteria, modules to read and write | Never edited once done |
| SPEC | A specification too big for a module (durable) or a task's plan (transient) | Transient ones are incorporated or archived at closure |
| ADR | **Why** a choice was made | Never edited once accepted; replaced by a new ADR |
| Constitution | The agent's rules | Rarely |
| Content | Optional resources in `mwp/content/`: icons, images, design files, documents | When resources are added |

**IDs**

| ID | What | Scope |
|---|---|---|
| `T-0043` | Task | Project |
| `SP-0003` | SPEC | Project |
| `ADR-0002` | Decision | Project |
| `TEST-0007` | Test | Project |
| `FR-01` / `NFR-01` | Functional / non-functional rule | Module, cited as `order#FR-01` |
| `AC-1` | Acceptance criterion | Task |

Project IDs are global, sequential, have at least 4 digits and are never reused. A company work code (Jira, ticket) goes in the task's `external:` field, never in `id:`.

**Front matter** is the contract with the CLI. Every module, task, SPEC and ADR starts with YAML, and files cite each other by ID, never by path. Files starting with `_` are templates and are ignored.

## 4. The workflow

### 4.1 Document a module

For code that already exists, ask the agent:

> Use the module skill to create the modules for this project.

It surveys the code, **stops to show the proposed modules** (ID, type, code and test folders, dependencies), and only writes them after you approve. It ends with a report: what it created, rules without tests, suspected bugs, and anything that needs your decision. It never changes code or tests.

To write one by hand, copy `mwp/modules/_template.md` to `mwp/modules/order.md` (subfolders such as `domain/order.md` are fine) and fill the front matter:

```yaml
---
kind: module
id: order
type: domain                 # ui | domain | infra
summary: Order creation and status rules
code: [src/order/]
tests: [tests/order/]
specs: []
depends_on: [customer]
updated: 2026-11-03
---
```

Register each test in the `## Tests` table with its ID. The test name in code starts with the same ID (`TEST-0007: 5% off over R$500`), which is how the CLI finds it.

Then check the module and refresh the map:

```
$ mwp remap
ERROR mwp/modules/order.md: `depends_on` cites `customer`, no module has that id
1 error(s), 0 warning(s)
map.md updated (1 modules)
```

### 4.2 Create a task

```
$ mwp task new "Volume discount" --modules order --reads customer --external PROJ-1234
create mwp/tasks/open/T-0001-volume-discount.md (size M, status: open)
```

Fill **Goal**, **Scope** and **Acceptance criteria**. Each criterion says how it is verified:

| Type | Verified by |
|---|---|
| `[auto: TEST-0007]` | The test passes |
| `[manual]` | The steps after `Verify:` |
| `[regression]` | All tests of the touched modules pass |
| `[doc]` | Module and changelog reflect the change |

Sizes: **S** uses the short template; **M** writes tests first; **L** also requires a plan approved by a person before implementing.

Before handing the task off, check what the agent will load:

```
$ mwp context T-0001
mwp/constitution.md                                        1010  always
mwp/tasks/open/T-0001-volume-discount.md                    240  always
mwp/modules/order.md                                        610  modules
mwp/modules/customer.md                                     480  reads
total                                                      2340  tokens (estimate)
```

If the total is large, split the task.

### 4.3 Hand it to the agent

> Execute task T-0001

You can also use the company code (*"Execute PROJ-1234"*). The agent follows the skill:

1. `mwp context T-0001 --paths`, then reads only those files.
2. Sets `status: in_progress`, creates the branch `task/T-0001-volume-discount`.
3. Size L: writes the plan and stops for approval.
4. Writes the tests first (`mwp next TEST --reserve`), confirms they fail.
5. Implements, only in the modules listed in `modules:`.
6. Verifies every criterion.
7. Updates the modules and runs `mwp remap`.
8. Fills `## Closure` with evidence, sets `status: done`, moves the file to `tasks/done/`, runs `mwp lint`.

If a `[manual]` criterion could not be checked, the task goes to `status: review` and a person confirms it. The agent stops and asks when the task contradicts a module or an accepted ADR, when the code contradicts its module, or when an existing test fails and seems to contradict the task.

Rework is always a new task with `refs: [T-0001]`.

### 4.4 Record a decision

```
$ mwp adr new "Store money in cents" --modules order
create mwp/decisions/ADR-0001-store-money-in-cents.md (status: proposed)
```

The agent creates ADRs as `proposed`. Only a person changes them to `accepted`. To replace a decision:

```
$ mwp adr new "Store money as decimal" --supersedes ADR-0001
create mwp/decisions/ADR-0002-store-money-as-decimal.md (status: proposed)
update mwp/decisions/ADR-0001-store-money-in-cents.md (status: superseded, superseded_by: ADR-0002)
```

### 4.5 Keep modules in sync

Modules drift when code changes outside the workflow. Every so often, or before a large task, ask:

> Use the module skill to audit the modules.

For each module the agent compares rules, tests, contracts and dependencies with the code, and checks the code against the accepted ADRs. It fixes factual drift (paths, dependencies, signatures, unregistered tests) and **asks** about the rest: a rule that differs from the code, behavior with no rule, code that violates an ADR. You can limit it: *"audit the order and payment modules"*.

### 4.6 Use resources

Put resources the agent may need in `mwp/content/`: icons, images, Figma files or exports, PDFs, reference documents, sample data. Subfolders by type keep it easy to browse:

```
mwp/content/
├── icons/
├── images/
├── design/      # .fig files, exports, links to Figma
└── docs/        # PDFs, client specs, reference material
```

- Cite a resource in the task by path: *"Use `mwp/content/icons/cart.svg` on the checkout button."*
- The agent looks in `content/` first, opens only what the task needs, and copies a resource into the code's asset folder when the app uses it. `content/` keeps the reference copy.
- If a resource is missing, the agent asks instead of inventing one.
- The folder is optional, and the CLI ignores it: `lint`, `map` and `next` never read it.

### 4.7 See where things stand

```
$ mwp task-list
T-0001   in_progress  M  Volume discount [PROJ-1234]
T-0002   open         S  Fix rounding

$ mwp adr list --status accepted
ADR-0001  accepted    2026-11-03 Store money in cents
```

## 5. The agent skills

`mwp init` installs two skills in `.claude/skills/` (skip them with `--no-skill`).

### `mwp`: the workflow

Source: [`src/skill/mwp/SKILL.md`](../../src/skill/mwp/SKILL.md). It teaches the agent:

- which files exist and which to ignore (templates, done tasks, superseded ADRs);
- the ID rules, including searching with a word boundary (`\bT-0043\b`, because `TEST-0043` contains `T-0043`);
- the steps to execute, create and close a task, and the CLI command for each step;
- how to find and use resources in `mwp/content/`;
- when to stop and ask;
- the manual equivalent of every command, for projects without the CLI.

### `module`: modules from the code

Source: [`src/skill/module/SKILL.md`](../../src/skill/module/SKILL.md). Two modes:

| Mode | Ask | What it does |
|---|---|---|
| Create | *"Create the modules for this project"* | Surveys the code, proposes boundaries and stops for approval, writes the modules, runs `remap` and `lint` |
| Audit | *"Audit the modules"* | Compares each module with its code, tests and ADRs; fixes factual drift; reports the rest |

Both end with a report that cites a file and line, or a rule ID, for every finding. The skill writes only in `mwp/modules/`: it never changes code, tests or accepted ADRs. Run it inside a task (size L) so changelog rows have a task ID.

The skill explains **how** to work; `mwp/constitution.md` holds the **rules**. If the two disagree, the constitution wins. Do not copy rules into the skill, `AGENTS.md` or `CLAUDE.md`: that creates two sources of truth.

**Other agents.** `AGENTS.md` points any agent to the constitution, which is enough to follow the format. If your agent supports skills, copy both `SKILL.md` files to its skills folder.

## 6. CLI reference

`mwp` finds the project by looking for `mwp/` in the current folder and its parents. Every command exits with code 0 on success and 1 on failure.

### `mwp init [dir]`

Creates the structure in `dir` (default: current folder). Never overwrites.

| Option | Effect |
|---|---|
| `--name <name>` | Fills the project name in `PROJECT.md` |
| `--no-skill` | Does not install the skills in `.claude/skills/` |

### `mwp task new <title>`

Creates `mwp/tasks/open/T-XXXX-slug.md` from the template, with the next task ID reserved in the ledger and today's date in `created:`. Warns if a cited module or task does not exist.

| Option | Effect |
|---|---|
| `--size S\|M\|L` | Task size (default `M`); `S` uses the short template |
| `--modules a,b` | Modules the task may write |
| `--reads a,b` | Modules the task only reads |
| `--refs T-XXXX` | Done tasks the agent may read |
| `--external CODE` | Company work code(s) |

### `mwp task-list` (or `mwp task list`)

Lists tasks with ID, status, size, title and external codes. Open tasks by default.

| Option | Effect |
|---|---|
| `--status <s>` | Only this status (`open`, `in_progress`, `blocked`, `review`, `done`) |
| `--module <id>` | Only tasks that write this module |
| `--all` | Includes done tasks |

### `mwp context <T-XXXX | code>`

Lists the files the agent loads for the task: the constitution, the task, and what it cites in `modules:`, `reads:`, `specs:`, `decisions:` and `refs:`, with a token estimate (about 4 bytes per token). Prints `MISSING` and exits 1 for references that do not exist.

| Option | Effect |
|---|---|
| `--paths` | Prints only the paths, one per line |

### `mwp next <T | SP | ADR | TEST>`

Prints the next free ID: the highest number found in MWP files, module test folders and the ledger, plus one. The constitution, the skill and the templates are not scanned, because their example IDs are not real.

| Option | Effect |
|---|---|
| `--reserve` | Records the number in `.mwp/ledger.json`, so it is never handed out again, even if the file is deleted |

`task new` and `adr new` reserve their IDs automatically.

### `mwp adr new <title>` / `mwp adr list`

| Option | Effect |
|---|---|
| `--modules a,b` | (`new`) Modules governed by the decision |
| `--supersedes ADR-XXXX` | (`new`) Marks that ADR `superseded` with `superseded_by:` and today's date |
| `--status <s>` | (`list`) Only this status |

### `mwp remap`

Rechecks every module and regenerates the table in `map.md`:

- `code:` and `tests:` paths exist;
- `depends_on:` and `specs:` cite existing IDs;
- every TEST in the `## Tests` table appears in a file under `tests:`.

| Option | Effect |
|---|---|
| `--dry-run` | Checks only; says whether `map.md` would change |

### `mwp map`

Prints the module table built from the module front matter.

| Option | Effect |
|---|---|
| `--write` | Writes the table between `<!-- mwp:map:begin -->` and `<!-- mwp:map:end -->` in `map.md`; text outside the markers is kept |
| `--check` | Exits 1 if `map.md` is out of date |

### `mwp lint`

Checks the whole project. It verifies only what needs no human judgment:

| Check | Level |
|---|---|
| Known `kind`, required fields, valid `status` / `size` / `lifetime` values | error |
| ID format (`T-`, `SP-`, `ADR-` plus at least 4 digits) and duplicate IDs | error |
| References to modules, tasks, SPECs and ADRs that do not exist | error |
| `refs:` citing a task that is not done | error |
| Task status does not match its folder; done task with an empty `## Closure` | error |
| Superseded ADR without a valid `superseded_by:` | error |
| SPEC `owner:` is not a module or task | error |
| Module checks from `remap` | error |
| `map.md` out of date | error |
| Module `type` outside `ui` / `domain` / `infra`; file name not starting with the task ID | warning |
| Over the size limits (PROJECT ~60, constitution ~150, module ~200, task ~80 lines) | warning |

| Option | Effect |
|---|---|
| `--strict` | Also exits 1 on warnings |

## 7. Continuous integration

Two commands are enough to keep the documentation honest:

```bash
mwp lint
mwp map --check
```

`lint` already includes the map check; run `map --check` alone if you want a fast step.

## 8. Troubleshooting

| Message | Meaning |
|---|---|
| `no mwp/ folder here or in a parent folder` | Run the command inside the project, or `mwp init` first |
| `MISSING ADR-0009 (decisions)` | The task cites an ID that no file has. Create it or fix the task |
| `TEST-0007 is in ## Tests but not found under tests paths` | The test was registered but its code does not contain the ID, or `tests:` points to the wrong folder |
| `map.md: out of date` | A module changed. Run `mwp remap` |
| `status is done but ## Closure is empty` | Fill the closure with evidence before closing |
| `` `status: done` but the file is in tasks/open `` | Move the file to `tasks/done/` (or fix the status) |
