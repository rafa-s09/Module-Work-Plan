# The MWP philosophy

MWP is not a safety net that prevents mistakes. It is a **production line**: a way of working that keeps a system healthy over a long life, with humans and agents each doing the part they do best.

Humans make mistakes. Agents make mistakes. That will not change. What MWP changes is where a mistake can come from, how fast it shows up, and how cheaply it is fixed.

## The premise: pay once, then run

Most of the cost of MWP is at the start: deciding the module boundaries, writing the rules, registering the tests, recording the decisions that matter. That is real work, and it needs people who know the business.

After that, the shape of the work changes. In a mature system (an ERP, a long-running product) the business rules are far more stable than the code. Most days nobody rewrites how an order is priced or how an invoice is closed. The work is **tasks**: a new report, a field, a fix, an integration. A module is touched only when a task changes the behavior it describes.

So the steady state is:

- **many tasks**, small and delegated;
- **few module changes**, made in the same task that changes the behavior;
- **rare decisions**, recorded once as ADRs and then respected.

Stable does not mean frozen. Laws, taxes and contracts change, and some modules will change often. The point is that a module changes *when a rule changes*, not every time someone writes code.

## The cycle

```
Structure  →  Delegate  →  Execute  →  Review
    ↑                                     │
    └─────────────────────────────────────┘
```

| Stage | Who | What happens | What it produces |
|---|---|---|---|
| **Structure** | Maintainers | Define modules, rules, tests and decisions; keep them true | Modules, ADRs, constitution |
| **Delegate** | Task author | Turn a need into a small task with a clear manifest and verifiable criteria | A task in `tasks/open/` |
| **Execute** | Agent | Load only the manifest, test first, implement, update the modules, close with evidence | Code, tests, updated modules, closure |
| **Review** | Reviewer | Check the evidence, the manual criteria and the module changes; accept or open a new task | A done task, or a follow-up task |

The review feeds the next round of structure: a module that turned out incomplete, a rule that was wrong, a decision that needs revisiting. Then the cycle repeats.

The CLI supports every stage but owns none: `task new` and `context` help delegation, `next` and `remap` help execution, `lint` helps review. It checks what needs no judgment, so people only spend judgment where it is needed.

## Who does what

| Role | Responsible for | Never |
|---|---|---|
| **Maintainers** (one or two engineers who know the business rules) | The truth of the modules, accepting ADRs, the constitution, auditing modules against the code | Let a module drift because "the code is the documentation" |
| **Task author** (often a maintainer) | Goal, scope, the manifest (`modules:`, `reads:`), criteria that say how they are verified, the size | Write vague criteria or leave the manifest empty |
| **Agent** | Executing the task inside its manifest, writing tests first, updating modules, closing with evidence, proposing ADRs, stopping on conflicts | Change an existing test to make it pass, edit a done task, accept an ADR, resolve a conflict silently |
| **Reviewer** (a person) | Checking the closure and the manual criteria, approving plans for size L | Approve a closure without evidence |
| **CLI** | IDs, the map, references, closures, registered tests | Judge whether a rule or a criterion is right |

### Why so few maintainers

The modules are the single source of truth for how the system behaves. A source of truth with many owners stops being one: rules get written twice, in different words, with different exceptions.

One or two engineers with a strong grasp of the business rules can keep a large system coherent, because they do not write most of the code. They write and protect the **rules**. Everyone else, people and agents, works through tasks against those rules.

This is also where the effort must be disciplined. The initial structure is only worth its cost if it stays true. A module that is not updated is worse than no module, because it is trusted.

## Errors have an address

MWP does not try to make errors impossible. It makes every error **traceable to where it was born**:

| The error came from | What it looks like | Where it is fixed |
|---|---|---|
| **The task** | Wrong goal, missing module in the manifest, criteria that did not capture the need | Task author: a new task with `refs:` |
| **The module** | A rule that was wrong or outdated, a missing pitfall | Maintainer: the module, in a new task |
| **The execution** | The agent did not meet a criterion, or the evidence does not hold | Review: back to the agent, or a new task |
| **A decision** | An ADR that no longer fits | Maintainers: a new ADR that supersedes it |

Because tasks are immutable, decisions are versioned and every criterion carries its evidence, you can always answer *"why does the system do this?"* and *"when did this change, and who asked for it?"*. That answer is what makes the system trustworthy, not the absence of mistakes.

A fix is always a **new** record (a task, an ADR) and never a rewrite of an old one. The history stays honest, and the next person or agent can learn from it.

## Trust comes from the line, not from perfection

When a task goes wrong, the question is never "who failed?" but "which link failed?":

- Was the task passed correctly?
- Was the module true?
- Did the execution meet the criteria, with evidence?
- Did the review check what it had to check?

Each link has an owner and a visible artifact, so each failure points to one place to improve. Over time the line gets stronger: tasks get clearer, modules get more precise, and agents get better context. That is how a system keeps a long life.

## What MWP does not try to do

- **Prevent every error.** It makes errors visible, local and cheap to fix.
- **Replace engineering judgment.** Boundaries, rules and decisions are human work. Agents propose; people decide.
- **Document everything up front.** Start with the module you will touch first. Structure grows with the work.
- **Pay off in the first week.** The return comes from the hundredth task, not the first.

## When it is not worth it

A prototype, a throwaway script or a project that will live for a few weeks does not need modules and ADRs. MWP pays off when a system has a long life, real business rules and work that can be delegated. That is exactly the kind of system where the initial effort is small compared to what it saves.

## The short version

1. **Structure** carefully, with few owners who know the business.
2. **Delegate** small tasks with clear manifests and verifiable criteria.
3. **Execute** inside the manifest, with tests first and evidence at the end.
4. **Review** the evidence, and turn every problem into a new task.
5. **Repeat.** Keep the modules true, and the line holds.
