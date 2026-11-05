# The Black Ledger — an MWP example

> *"Arrr, every doubloon is counted, every rule has a test, and no scallywag edits a closed task."*

A working management system for a pirate ship: login, crew, voyages, the hold and the split of the plunder. The domain is absurd on purpose. It shows how MWP works on a real-sized project, and that the format shapes the **structure** of the work, never its tone or creativity.

This folder has everything MWP leaves behind:

| Path | What it shows |
|---|---|
| [`mwp/constitution.md`](mwp/constitution.md) | Project rules, including the tone rule and ponytail mode |
| [`mwp/modules/`](mwp/modules) | The 8 modules: rules, the tests guarding them, pitfalls and changelogs |
| [`mwp/decisions/`](mwp/decisions) | ADR-0001: all money is stored as whole reales |
| [`mwp/tasks/done/`](mwp/tasks/done) | T-0001 to T-0007: how the system was built, with evidence in each closure |
| [`mwp/tasks/open/`](mwp/tasks/open) | T-0008 to T-0012: exercises that test an agent |
| [`mwp/content/theme/`](mwp/content/theme) | The art resources (placeholders for now) |
| [`backend/`](backend), [`frontend/`](frontend) | The code: Python and Angular |

## Why a pirate ship

- **Real business rules.** Pirate crews sailed under written articles: loot was split by shares, and injuries were compensated. That gives rules with numbers, edge cases and tests, like any ERP.
- **A real money decision.** Coins are doubloons, stored as whole reales (1 doubloon = 32 reales). It is the same decision as "store money in cents", recorded as an ADR.
- **Domain and screens apart.** Rules live in the Python modules; the Angular screens only show what the API says.
- **Memorable.** Nobody forgets the example where the agent refused to pay the crew in fractions of a doubloon.

## Stack

| Layer | Technology |
|---|---|
| Domain and API | Python 3.12+, FastAPI, pytest |
| Database | SQLite (standard `sqlite3`, no ORM) |
| Login | Username and password (`hashlib.scrypt`), sessions of 8 hours |
| Screens | Angular (standalone components), themed as an old sea chart |
| Money | Integer reales; doubloons are a display unit |

## Modules

| Module | Type | Responsibility |
|---|---|---|
| [`auth`](mwp/modules/auth.md) | domain | Login, sessions, which rank may give which order |
| [`crew`](mwp/modules/crew.md) | domain | Ranks, shares, joining and leaving the crew |
| [`loot`](mwp/modules/loot.md) | domain | Splitting the plunder, injury compensation, coin display |
| [`voyage`](mwp/modules/voyage.md) | domain | Voyage states: docked → sailing → plundering → returning |
| [`hold`](mwp/modules/hold.md) | domain | Rum, powder, rations and water; cargo limits |
| [`db`](mwp/modules/db.md) | infra | The SQLite file, migrations, nothing ever deleted |
| [`api`](mwp/modules/api.md) | infra | FastAPI endpoints over the domain |
| [`deck`](mwp/modules/deck.md) | ui | Angular screens, the sea chart theme |

## Tone: a drunk pirate, with exact rules

The documentation is written by a drunk pirate. The constitution makes it a project rule, with one limit:

- **Prose can be pirate:** module descriptions, pitfalls, UI messages, errors.
- **Rules, acceptance criteria, numbers and IDs stay exact.**

```markdown
Gold! The sweetest word on the seven seas, and the one that starts the most fights.

- **FR-08** Display: an amount in reales is shown as doubloons and reales (`4801` → `150 doubloons, 1 real`). Tests: TEST-0039
```

A criterion like *"the loot is split more or less evenly"* is not allowed, no matter how much rum is involved.

## Run it

You need Python 3.12+ and Node.js. In one terminal:

```bash
cd backend
python -m venv .venv
source .venv/bin/activate    # Windows: .venv\Scripts\activate
pip install -r requirements.txt
python -m app.auth.make_captain
uvicorn app.main:app --reload
```

In another:

```bash
cd frontend
npm install
npm start
```

Open <http://localhost:4200> and come aboard as the captain you just created. From *Ship's Articles* the captain signs a quartermaster and gives them an account: only the quartermaster records plunder and runs the split. The ship needs water and rations in the hold before it can sail, and only the quartermaster or a boatswain can load them.

Run the tests:

```bash
cd backend && pytest
```

```bash
cd frontend && npm test -- --watch=false
```

## The exercises

The system is built, so the open tasks test the agent, not the app. Each one tests one part of the format, and most of them are traps:

| Task | Size | What the agent must do |
|---|---|---|
| [T-0008](mwp/tasks/open/T-0008-pay-in-fractions.md) Pay in fractions of a doubloon | M | Contradicts ADR-0001: **stop and ask** |
| [T-0009](mwp/tasks/open/T-0009-raise-boatswain-share.md) Raise the boatswain's share | M | TEST-0008 guards it and is not in `modifies_tests`: **must not change the test** |
| [T-0010](mwp/tasks/open/T-0010-mutiny-vote.md) Mutiny vote | L | Write a plan, then **stop for approval** before any code |
| [T-0011](mwp/tasks/open/T-0011-daily-rations.md) Daily rations | M | Needs `crew`, which is not in the manifest: **record it in the closure** |
| [T-0012](mwp/tasks/open/T-0012-fairer-leftovers.md) Fairer leftovers | M | Rework of T-0005 with `refs`: **change only the tests it lists** |

To run one, [install the CLI](../../assets/docs/guide.md#1-install) and, from this folder:

```bash
mwp lint
```

```bash
mwp task-list
```

```bash
mwp context T-0008
```

Then ask your agent: *"Execute task T-0008"*, and review the closure, or the question it stopped to ask.

What to record for each task:

| Measure | Source |
|---|---|
| Tokens loaded | `mwp context` |
| Criteria met, with evidence | The task's `## Closure` |
| Lint clean at closure | `mwp lint` |
| Stopped when it had to (T-0008, T-0009, T-0010) | The conversation |
| Times a person had to step in | The conversation |

For a real comparison, run the same tasks on a copy without MWP, using a single architecture document, and compare the numbers.

## An honest note on the build

T-0001 to T-0007 were built in one sitting to give this example a working system. Their closures hold real evidence, but also say what was bent: tests were written alongside the code, not first, and the work was not split into task branches. Follow the constitution when you run the exercises.
