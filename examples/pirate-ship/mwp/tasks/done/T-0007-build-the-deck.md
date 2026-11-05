---
kind: task
id: T-0007
status: done
size: L
created: 2026-11-05
modules: [deck]
reads: [api]
specs: []
refs: []
decisions: [ADR-0001]
modifies_tests: []
external: []
---

# T-0007 — Build the deck: the Angular screens and the chart theme

## Goal
Where the crew stands. Every screen of the module, with the sea chart theme, and no business rule of its own.

## Scope
- Includes: all screens, `ApiService`, the guard, the theme, placeholder art.
- Excludes: the real theme art, which the owner will bring to `mwp/content/theme/`.

## Acceptance criteria
- [x] AC-1 [auto: TEST-0059, TEST-0060] Sessions and redirects.
- [x] AC-2 [auto: TEST-0061] Buttons follow orders.
- [x] AC-3 [auto: TEST-0062] Money as given by the API.
- [x] AC-4 [auto: TEST-0063, TEST-0065] Refusals in a bottle.
- [x] AC-5 [auto: TEST-0064] Where the token lives.
- [x] AC-6 [manual] A whole voyage works in a browser. Verify: log in as each rank, sail, plunder, split.
- [x] AC-7 [manual] NFR-01 contrast. Verify: compute the ratios of the text token pairs.
- [x] AC-8 [manual] NFR-03 at 360 px. Verify: no sideways scroll; tables stack.
- [x] AC-9 [doc] `deck` module and changelog updated.

## Closure
- **Done:** All screens, the theme tokens and two placeholder SVGs. Fixed during the browser run: a refused select stayed on the refused value (TEST-0065).
- **Acceptance evidence:** `cd frontend && npm test`: 7 passed; `npm run build` passes. AC-6: in the browser pane on 2026-11-05, ran a voyage as captain, quartermaster and sailor; the split showed 10003 reales as 312 doubloons, 19 reales, chest 2 reales. AC-7: ink on parchment 12.7:1, ink on deep parchment 10.6:1, sea on parchment 7.3:1, sea on deep parchment 6.1:1, parchment on sea 7.3:1, parchment on seal 6.0:1. AC-8: at 360 px, scroll width 360, tables stacked.
- **TESTs created:** TEST-0059 to TEST-0065.
- **SPECs:** None.
- **Modules read outside the manifest:** None.
- **Deviations from the plan:** No plan was stopped for approval: the owner's request to finish the example stood in for it. Built in one sitting to finish this example, on the request of the project owner. Tests were written alongside the code, not first; no task branch or commits, the owner commits.
- **Follow-ups / suggested tasks:** Not verified by hand: NFR-02 across every screen (only native controls are used, and the login was run by keyboard) and NFR-04. Replace the placeholder art.
