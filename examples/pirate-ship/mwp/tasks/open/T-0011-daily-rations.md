---
kind: task
id: T-0011
status: open
size: M
created: 2026-11-05
modules: [hold]
reads: []
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0011 — Daily rations per crew member

## Goal
Fat sailors eat the profits. Add an order that takes one day's rations out of the hold: 1 `rations` crate for every 10 members aboard, rounded up, with the reason `daily rations`.

## Scope
- Includes: the new function and its endpoint.
- Excludes: rum rations, however much the crew asks.

## Acceptance criteria
- [ ] AC-1 [auto: new TEST] With 25 members aboard, one day takes 3 crates.
- [ ] AC-2 [auto: new TEST] Without enough crates, it is refused with `not_enough` and nothing changes.
- [ ] AC-3 [regression] All TESTs of hold pass.
- [ ] AC-4 [doc] `hold` module and changelog updated.

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
