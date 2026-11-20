---
kind: task
id: <T-XXXX>
status: open            # open | in_progress | blocked | review | done
size: M                 # XS | S | M | L | XL | XXL (L and up require an approved plan)
created: <YYYY-MM-DD>
modules: []             # modules the task may write
reads: []               # extra modules to read, not write
specs: []
refs: []
decisions: []
modifies_tests: []      # existing TESTs the task may change; justify each in Scope
external: []            # company work codes (Jira, ticket, OS...); optional
---

# <T-XXXX> — <Title>

## Goal
<What and why, in 2 to 4 lines.>

## Scope
- Includes: <...>
- Excludes: <...>

## Acceptance criteria
- [ ] AC-1 [auto: TEST-XXXX] <...>
- [ ] AC-2 [manual] <...> Verify: <...>
- [ ] AC-3 [regression] All TESTs of <modules> pass.
- [ ] AC-4 [doc] <Rules and TESTs registered in the module; changelog updated.>

## Plan
> Required for size L and up: the agent fills it in and STOPS until approval.
> XL: the plan goes into a transient SPEC. XXL: the plan is the list of smaller tasks.
> Optional for M. Over ~30 lines, move it to a transient SPEC.

## Closure
> Filled in by the agent when finishing.
- **Done:**
- **Acceptance evidence:**
- **TESTs created:**
- **SPECs:**
- **Modules read outside the manifest:**
- **Deviations from the plan:**
- **Follow-ups / suggested tasks:**
- **Replanning:**
> Only when replanned: why the plan failed and the new tasks. Mark each criterion not met `- [-] ... — failed: <reason>`.
