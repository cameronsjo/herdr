## ADDED Requirements

### Requirement: A pinned agent leaves its group
A pinned agent SHALL be drawn in the pinned run at its pin order and MUST leave its workspace or token group, even when its name is its token group's value, so it does not also head that group. The needs-me filter runs before pinning, so while the filter is on a pinned agent that is not blocked is hidden.

#### Scenario: Pinned token-group leader
- **WHEN** `group_by = { token = "parent" }` and agent `three`, whose name is its `parent` value, is pinned
- **THEN** `three` is drawn in the `pinned` run and the `three` group lists its other members

#### Scenario: Needs me hides an idle pin
- **WHEN** the needs-me filter is on and a pinned agent is idle
- **THEN** that agent is not listed
