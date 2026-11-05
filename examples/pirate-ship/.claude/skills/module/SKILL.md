---
name: module
description: Create or audit MWP modules from the code. Use when a project has code but no modules (or is missing some) and asked to "document the modules", "bootstrap MWP", "create modules from the code"; or when asked to validate, audit, sync or update existing modules against the code and the ADRs. Produces module files and a report of every finding.
---

# MWP modules from the code

> Draft. Follow `mwp/constitution.md` and the `mwp` skill for everything not covered here.

A module describes how a part of the system works **today**. This skill builds modules from the code (**create**) or checks existing modules against the code and the ADRs (**audit**). Both end with a report.

## Ground rules

- **Never change code or tests.** This skill only writes files in `mwp/modules/` (and `map.md` through `mwp remap`).
- **Never edit an accepted ADR.** If code contradicts one, report it.
- **Describe, don't judge.** A rule states what the code does. If the code looks wrong, do not write the bug as a rule: report it as a suspected bug.
- **Never resolve a conflict silently.** When the module and the code disagree on behavior, report it and ask which one is right. Only factual drift (paths, dependencies, signatures, test registration) is fixed directly.
- **Run it as a task.** If no task covers the work, create one (`mwp task new "Document modules" --size L`) so changelog rows have a task ID. Size L: the proposal below is the plan, and you stop for approval.

## Create: modules for existing code

1. **Load context.** Constitution, `PROJECT.md`, `map.md`, the module list (`mwp map`) and the decisions in force (`mwp adr list --status accepted`, then read the ones that govern code you will document).
2. **Survey, don't read everything.** Look at the folder tree, build files, entry points, test folders and imports between folders. Open files only to confirm what a folder does.
3. **Propose boundaries.** One module per cohesive responsibility:
   - `domain`: business rules and states. Rules live here.
   - `ui`: screens, CLI, API handlers. They cite domain rules (`see order#FR-02`), never repeat them.
   - `infra`: database, external services, queues, files.
   - A module over ~200 lines must be split, or its detail moved to a durable SPEC.
4. **Stop and show the proposal** as a table (ID, type, summary, `code:`, `tests:`, `depends_on:`) with the reason for each boundary. Write nothing until a person approves it.
5. **Write each module** from `mwp/modules/_template.md`:
   - Front matter: `code:` and `tests:` point to real folders; `depends_on:` comes from real imports; `updated:` is today.
   - **Functional / non-functional rules:** one observable behavior each (inputs, outputs, limits, errors), with `FR-01`, `NFR-01` numbering per module. Cite the test that covers it, or write `Tests: none`.
   - **States and transitions, contracts:** from the code (exported functions, endpoints, events).
   - **Tests table:** only tests whose name already starts with a `TEST-XXXX` ID. Do not rename tests: list the others in the report and suggest a task to tag them.
   - **Related decisions:** ADRs with this module in `modules:` or that clearly govern its code.
   - **Known pitfalls:** only what the code or comments show (workarounds, `TODO`, `HACK`, surprising behavior).
   - **Changelog:** one row with today's date, the task ID and "Module created from code".
6. **Check.** `mwp remap`, then `mwp lint`. Fix what lint reports inside `mwp/modules/`.
7. **Report** (format below).

## Audit: existing modules against the code

Audit every module, or only the ones asked for. For each module:

1. **Read** the module, the files under its `code:` and `tests:`, and its ADRs (listed in `## Related decisions` or citing it in `modules:`).
2. **Mechanical checks:** `mwp remap --dry-run` (paths, `depends_on`, registered tests).
3. **Compare** the module with the code:

   | Check | Question |
   |---|---|
   | Rules | Is each FR/NFR still what the code does? |
   | Missing rules | Does the code have behavior no rule describes? |
   | Tests | Does each registered test exist and cover the rule it claims? Are there tests with IDs that are not registered? |
   | Contracts | Do signatures, endpoints and events match? |
   | Dependencies | Does `depends_on` match the real imports? |
   | ADRs | Does the code follow every accepted ADR that governs it? |
   | Pitfalls | Are listed pitfalls still true? |
   | Size | Is the module over ~200 lines? |

4. **Act on each finding:**

   | Finding | Action |
   |---|---|
   | Wrong path, missing or extra `depends_on`, outdated signature | Fix the module |
   | Test with an ID that exists but is not registered | Register it in `## Tests` |
   | Rule differs from the code | **Ask.** Report both versions; do not pick one |
   | Behavior with no rule | Report it; add the rule only if a person confirms it is intended |
   | Code violates an accepted ADR | **Report as a conflict.** Change nothing |
   | Registered test missing or not covering its rule | Report it |
   | Module too large | Propose a split or a durable SPEC |

5. If the module changed: set `updated:` to today and add a changelog row with the task ID. Then `mwp remap` and `mwp lint`.

## Report

Write the report in the chat. When working inside a task, also summarize it in the task's `## Closure`.

```markdown
# Module report — <create | audit> — <YYYY-MM-DD>

## Summary
<N modules created / audited, N updated, N need a decision.>

## Modules
| Module | Status | Changes made |
|---|---|---|
| order | updated | depends_on: + customer; TEST-0012 registered |
| payment | needs decision | 2 conflicts |

## Need a decision
- **order#FR-02** — module: 5% off over R$500; code: 5% off from R$500 (`src/order/discount.py:14`). Which is right?
- **ADR-0001** (money in cents) — `src/payment/gateway.py:30` uses float.

## Suspected bugs
- <file:line — what looks wrong and why.>

## Not covered by tests
- order#FR-03, payment#FR-01

## Suggested tasks
- Tag existing tests with TEST IDs in `tests/order/`.
- <...>
```

Every finding cites a file and line, or a module rule ID. Report what you checked and found, not what you assume.
