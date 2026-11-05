---
kind: module
id: deck
type: ui
summary: The deck. Angular screens that look like an old sea chart and a captain's logbook, with no business rules of their own.
code: [frontend/src/app, frontend/src/styles, frontend/src/styles.css, frontend/public/theme]
tests: [frontend/src/app/app.spec.ts]
specs: []
depends_on: [api]
updated: 2026-11-05
---

# deck — The Deck

Where the crew stands and looks at the ledger. It should feel like unrolling a sea chart on the captain's table: parchment, ink, a compass rose, rhumb lines running off to islands nobody has named yet. Pretty as a mermaid, but it decides nothing: every rule is the back's, and the deck only shows and asks.

## Functional rules
- **FR-01** Screens, their route and who sees them. Each form or button of an order follows FR-03.

| Screen | Route | Shows | Who |
|---|---|---|---|
| Board the Ship | `/login` | Username, password, "Come aboard" button | everyone |
| The Chart | `/` | Active or latest voyage on a sea chart, its state as a route, hold weight, chest balance | logged in |
| Ship's Articles | `/crew` | Crew roster: name, rank, shares, status; signing, accounts, own password | logged in |
| The Course | `/voyages` | Voyages and their states; buttons to change state | logged in |
| Below Deck | `/hold` | Item counts, weight against capacity as a filling barrel | logged in |
| Divide the Plunder | `/voyages/:id/plunder` | Plunder, compensations, the split lines and the chest part | logged in |

- **FR-02** Without a valid session, every route except `/login` goes to `/login`. A 401 from the API ends the session in the deck and goes to `/login`. Tests: TEST-0059, TEST-0060
- **FR-03** A button for an order the member's rank may not give (auth#FR-09) is not shown. The rank's orders come from `GET /api/me`. The back still checks; hiding is only courtesy. Tests: TEST-0061
- **FR-04** Money is shown with the `*_display` string from the API (loot#FR-08), never computed in the deck. Tests: TEST-0062
- **FR-05** An API error shows its `message` in a "message in a bottle" notice. The deck never shows the error code alone. A refused rank or status goes back to what the back says. Tests: TEST-0063, TEST-0065
- **FR-06** The session token is kept in memory and in `sessionStorage`, never in `localStorage` or a cookie the deck sets. Tests: TEST-0064

## Non-functional rules
- **NFR-01** Text and controls meet WCAG 2.2 AA contrast on the parchment background. The theme never wins over legibility. Verified by hand (see T-0007).
- **NFR-02** Every screen works with the keyboard alone, with a visible focus ring.
- **NFR-03** Usable from 360 px wide. The chart scales; tables become stacked cards below 600 px. Verified by hand (see T-0007).
- **NFR-04** Decorative effects (the turning compass) stop when `prefers-reduced-motion` is set.

## Theme: charts and ships
Design tokens are CSS custom properties in `frontend/src/styles/theme.css`. Components use the tokens, never raw colors.

| Token | Value | Use |
|---|---|---|
| `--parchment` | `#F2E6C9` | Page background, like an old chart |
| `--parchment-deep` | `#E4D3A8` | Cards and panels |
| `--ink` | `#2B2118` | Body text, lines, borders |
| `--sea` | `#1F4E5F` | Headers, links, buttons, focus ring |
| `--sea-shallow` | `#6E9CA8` | The water on the chart. Never for text on parchment |
| `--brass` | `#B08D3C` | Decoration: the barrel, the top bar's line. Never for text |
| `--seal` | `#9E2B25` | Danger and refusals, like a wax seal |
| `--kelp` | `#3F6B3A` | Reserved for success |

- **Type:** headings in *IM Fell English* (Google Fonts, with Georgia as the fallback); body, tables and numbers in a plain readable face with tabular figures.
- **Motifs:** compass rose (logo and login), rhumb lines and a faint grid in the page background, a dashed route line for the voyage states, double rope-like borders on cards.
- **Voyage on the chart:** the states (`docked`, `sailing`, `plundering`, `returning`) are stops on a dashed route; the ship icon sits on the current one. `sunk` shows the ship capsized where she went down.
- **Resources:** the reference copies live in `mwp/content/theme/`; a screen uses a copy in `frontend/public/theme/`. `compass-rose.svg` and `ship.svg` are simple placeholders until the real art arrives: replace them with the same names and nothing else changes. For any other resource, ask; never invent one.
- **Copy:** pirate English for labels and messages ("Come aboard", "Hoist the sails", "Divide the plunder"), but the meaning stays clear: a button says what it does.

## Contracts / interfaces
- Uses only the API in api#FR-06. One `ApiService` holds every call.
- The API runs on port 8000; `proxy.conf.json` sends `/api` there during `npm start`.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| TEST-0059 | FR-02 | Every guarded route goes to the login | frontend/src/app/app.spec.ts |
| TEST-0060 | FR-02 | A 401 ends the session | frontend/src/app/app.spec.ts |
| TEST-0061 | FR-03 | Buttons follow the rank's orders | frontend/src/app/app.spec.ts |
| TEST-0062 | FR-04 | Money shown as given by the API | frontend/src/app/app.spec.ts |
| TEST-0063 | FR-05 | A refusal shows its message, not its code | frontend/src/app/app.spec.ts |
| TEST-0064 | FR-06 | Where the token lives; sent as Bearer | frontend/src/app/app.spec.ts |
| TEST-0065 | FR-05 | A refused select goes back to the back's value | frontend/src/app/app.spec.ts |

## Related decisions
- [ADR-0001 — All money is stored as whole reales](../decisions/ADR-0001-whole-reales.md)

## Known pitfalls
- Parchment textures behind text kill contrast. Keep textures on margins and backgrounds, and test NFR-01 with the texture on.
- A beautiful old-style font for numbers makes 1 and 7 look alike. Numbers use the body face.
- `[ngModel]` does not redraw a select when the value did not change, so a refused choice would stay on screen. The rank and status selects set their value by hand after each answer.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
| 2026-11-05 | T-0007 | Built. The focus ring moved from brass to sea, because brass fails contrast on parchment. Theme art is placeholder. |
