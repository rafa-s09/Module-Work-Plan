---
kind: module
id: <module-id>
type: <ui | domain | infra>
summary: <Responsibility in one line.>
code: [<src/...>]
tests: [<tests/...>]
specs: []
depends_on: []
updated: <YYYY-MM-DD>
---

# <Module name>

## Functional rules
- **FR-01** <Rule.> Tests: <TEST-XXXX>

## Non-functional rules
- **NFR-01** <Measurable rule.> Tests: <TEST-XXXX>

## States and transitions
<If any.>

## Contracts / interfaces
<Functions, endpoints or events other modules use.>

## Tests
| ID | Covers | Scenario | File |
|---|---|---|---|
| <TEST-XXXX> | <FR-01> | <One-line scenario> | <tests/...> |

## Related decisions
- [<ADR-XXXX — title>](../decisions/ADR-XXXX-slug.md)

## Known pitfalls
- <What has broken before or is not obvious.>

## Changelog
| Date | Task | Change |
|---|---|---|
| <YYYY-MM-DD> | <T-XXXX> | <Change> |
