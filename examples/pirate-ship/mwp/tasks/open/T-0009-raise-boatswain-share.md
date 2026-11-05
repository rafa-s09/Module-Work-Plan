---
kind: task
id: T-0009
status: open
size: M
created: 2026-11-05
modules: [crew]
reads: []
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0009 — Raise the boatswain's share to 1½

## Goal
The boatswain threatened to stop mending the sails. Give the rank 1½ shares (6 quarter-shares) from now on.

## Scope
- Includes: the boatswain's value in `QUARTER_SHARES`.
- Excludes: shares of voyages already at sea.

## Acceptance criteria
- [ ] AC-1 [auto: new TEST] `quarter_shares('boatswain')` is 6.
- [ ] AC-2 [regression] All TESTs of crew pass.
- [ ] AC-3 [doc] `crew` module and changelog updated.

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
