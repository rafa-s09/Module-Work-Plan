---
kind: adr
id: ADR-0001
status: accepted
date: 2026-11-05
modules: [loot, api, deck, db]
supersedes: []
---

# ADR-0001 — All money is stored as whole reales

## Context
A crew will forgive a lost leg before it forgives a lost coin. Shares divide plunder into parts that rarely come out even, and the old coins do not divide evenly either: a doubloon is 32 reales, a piece of eight is 8. Store doubloons as decimals and the sums drift, a real at a time, until someone gets keelhauled over a rounding error.

## Options considered

| Criterion | Whole reales (integer) | Doubloons as float | Doubloons as decimal |
|---|---|---|---|
| Exact sums | Yes | No | Yes |
| Same in Python, SQLite and TypeScript | Yes | No | No: needs conversion at every border |
| A rule for what won't divide | Forced, and explicit | Hidden in rounding | Hidden in rounding |

## Decision
1. Every amount of money is stored, computed and sent through the API as a whole number of reales.
2. Doubloons and pieces of eight are display units. 1 doubloon = 32 reales; 1 piece of eight = 8 reales.
3. Floats, decimals and fractions of a real are never used for money, in any layer.
4. What does not divide evenly is decided by an explicit rule in `loot`, never by rounding.

## Consequences
- Sums are exact: `loot` can check that a split adds up to the plunder, to the real.
- Money columns are `INTEGER` and end in `_reales`.
- Amounts must be formatted for humans, and only the back does it, so the deck never does money math.
- Paying "a quarter of a doubloon" works only because it is 8 whole reales. Anything finer needs a new ADR.
