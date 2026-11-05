---
kind: module
id: deck
type: ui
summary: The deck. Angular screens that look like an old sea chart and a captain's logbook, with no business rules of their own.
code: []
tests: []
specs: []
depends_on: [api]
updated: 2026-11-04
---

# deck — The Deck

Where the crew stands and looks at the ledger. It should feel like unrolling a sea chart on the captain's table: parchment, ink, a compass rose, rhumb lines running off to islands nobody has named yet. Pretty as a mermaid, but it decides nothing: every rule is the back's, and the deck only shows and asks.

> Planned code: `frontend/src/app/`. Planned tests: `frontend/src/app/**/*.spec.ts`.

## Functional rules
- **FR-01** Screens, their route and who sees them:

| Screen | Route | Shows | Who |
|---|---|---|---|
| Board the Ship | `/login` | Username, password, "Come aboard" button | everyone |
| The Chart | `/` | Active voyage on a sea chart, its state as a route, hold weight, chest balance | logged in |
| Ship's Articles | `/crew` | Crew roster: name, rank, shares, status | logged in |
| The Course | `/voyages` | Voyages and their states; buttons to change state | logged in |
| Below Deck | `/hold` | Item counts, weight against capacity as a filling barrel | logged in |
| Divide the Plunder | `/voyages/:id/plunder` | Plunder, compensations, the split lines and the chest part | logged in |

- **FR-02** Without a valid session, every route except `/login` goes to `/login`. A 401 from the API ends the session in the deck and goes to `/login`.
- **FR-03** A button for an order the member's rank may not give (auth#FR-09) is not shown. The back still checks; hiding is only courtesy.
- **FR-04** Money is shown with the `*_display` string from the API (loot#FR-08), never computed in the deck.
- **FR-05** An API error shows its `message` in a "message in a bottle" notice. The deck never shows the error code alone.
- **FR-06** The session token is kept in memory and in `sessionStorage`, never in `localStorage` or a cookie the deck sets.

## Non-functional rules
- **NFR-01** Text and controls meet WCAG 2.2 AA contrast on the parchment background. The theme never wins over legibility.
- **NFR-02** Every screen works with the keyboard alone, with a visible focus ring.
- **NFR-03** Usable from 360 px wide. The chart scales; tables become stacked cards below 600 px.
- **NFR-04** Decorative effects (waves, a turning compass) stop when `prefers-reduced-motion` is set.

## Theme: charts and ships
Design tokens are CSS custom properties in `frontend/src/styles/theme.css`. Components use the tokens, never raw colors.

| Token | Value | Use |
|---|---|---|
| `--parchment` | `#F2E6C9` | Page background, like an old chart |
| `--parchment-deep` | `#E4D3A8` | Cards, table stripes, panels |
| `--ink` | `#2B2118` | Body text, lines, borders |
| `--sea` | `#1F4E5F` | Headers, links, the sea on the chart |
| `--sea-shallow` | `#6E9CA8` | Shallow water, secondary lines, disabled state |
| `--brass` | `#B08D3C` | Highlights, the compass rose, focus ring |
| `--seal` | `#9E2B25` | Danger and refusals, like a wax seal |
| `--kelp` | `#3F6B3A` | Success, the ship safe in port |

- **Type:** headings in a period display serif (for example *IM Fell English*); body, tables and numbers in a plain readable face with tabular figures. Numbers must always be easy to read.
- **Motifs:** compass rose (logo and loading), rhumb lines and a faint grid in the page background, a dashed route line for the voyage states, rope borders on cards, a wax seal for confirm dialogs, a ship's wheel icon for the menu.
- **Voyage on the chart:** the states (`docked`, `sailing`, `plundering`, `returning`) are stops on a dashed route; the ship icon sits on the current one. `sunk` shows the ship under the waves, in `--sea-shallow`.
- **Resources:** icons, illustrations, fonts and textures come from `mwp/content/theme/` (to be added). Copy what a screen uses into `frontend/src/assets/`; `content/` keeps the reference copy. If a resource is missing, ask; never invent one.
- **Copy:** pirate English for labels and messages ("Come aboard", "Hoist the sails", "Divide the plunder"), but the meaning stays clear: a button says what it does.

## Contracts / interfaces
- Uses only the API in api#FR-06. One `ApiService` holds every call.

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|

## Related decisions
- None yet.

## Known pitfalls
- Parchment textures behind text kill contrast. Keep textures on margins and backgrounds, and test NFR-01 with the texture on.
- A beautiful old-style font for numbers makes 1 and 7 look alike. Numbers use the body face.

## Changelog
| Date | Task | Change |
|---|---|---|
| 2026-11-04 | — | Module charted before the first task. |
