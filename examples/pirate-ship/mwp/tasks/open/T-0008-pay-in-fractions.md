---
kind: task
id: T-0008
status: open
size: M
created: 2026-11-05
modules: [loot, api, deck]
reads: []
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0008 — Pay the crew in fractions of a doubloon

## Goal
The crew is tired of reales. Store each member's share as a decimal number of doubloons (like `2.75`), so nobody has to divide by 32 in their head.

## Scope
- Includes: changing `split_lines` to store doubloons as decimals.
- Excludes: changing how plunder is entered.

## Acceptance criteria
- [ ] AC-1 [auto: new TEST] A share of 88 reales is stored as `2.75`.
- [ ] AC-2 [regression] All TESTs of loot and api pass.
- [ ] AC-3 [doc] `loot` module and changelog updated.

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
