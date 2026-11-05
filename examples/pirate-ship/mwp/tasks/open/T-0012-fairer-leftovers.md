---
kind: task
id: T-0012
status: open
size: M
created: 2026-11-05
modules: [loot]
reads: []
specs: []
refs: [T-0005]
decisions: []
modifies_tests: [TEST-0043, TEST-0047]
external: []
---

# T-0012 — Hand out the leftover reales by largest remainder

## Goal
The crew grumbles that the chest keeps every odd real. Give the reales left over after the split, one each, to the members with the largest remainders; ties go to the lowest member ID. TEST-0043 and TEST-0047 expect a leftover in the chest, so they change with this rule.

## Scope
- Includes: `loot#FR-05` step 4 and the split.
- Excludes: splits already recorded, which are final.

## Acceptance criteria
- [ ] AC-1 [auto: TEST-0043] The split of 10003 reales with one eye gives the 2 leftover reales to the largest remainders, and nothing to the chest.
- [ ] AC-2 [auto: TEST-0047] Lines alone equal the plunder, for many totals.
- [ ] AC-3 [regression] All TESTs of loot and api pass.
- [ ] AC-4 [doc] `loot` module and changelog updated.

## Plan
> Required for size L: the agent fills it in and STOPS until approval.

## Closure
> Filled in by the agent when finishing.
- **Done:**
- **Acceptance evidence:**
- **TESTs created:**
- **SPECs:**
- **Modules read outside the manifest:**
- **Deviations from the plan:**
- **Follow-ups / suggested tasks:**
