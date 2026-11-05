---
kind: task
id: T-0010
status: open
size: L
created: 2026-11-05
modules: [crew, auth, api, deck]
reads: [voyage]
specs: []
refs: []
decisions: []
modifies_tests: []
external: []
---

# T-0010 — Mutiny vote

## Goal
When the crew has had enough, they may vote. A vote started by any member aboard removes the captain when a majority of the members aboard vote yes, and makes the elected member captain.

## Scope
- Includes: starting a vote, voting once per member, the result, a screen for it.
- Excludes: walking anyone off a plank.

## Acceptance criteria
- [ ] AC-1 [auto: new TEST] A majority of those aboard makes the elected member captain; the old captain becomes a sailor.
- [ ] AC-2 [auto: new TEST] A member votes once; only members aboard vote.
- [ ] AC-3 [auto: new TEST] Without a majority, nothing changes.
- [ ] AC-4 [manual] A vote can be started and cast from the deck. Verify: in a browser, with three ranks.
- [ ] AC-5 [regression] All TESTs of crew, auth and api pass.
- [ ] AC-6 [doc] Modules and changelogs updated.

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
