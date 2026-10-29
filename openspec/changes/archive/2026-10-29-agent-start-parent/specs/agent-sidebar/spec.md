## ADDED Requirements

### Requirement: The agent a token group is named after leads it
Under `group_by = { token = NAME }` in grouped order, an agent whose name equals its group's token value SHALL be drawn first in that group, ahead of blocked members moved up by `blocked_first`; every other order is kept. Agent navigation and the multi-machine list MUST use the same order.

#### Scenario: Orchestrator heads its workers
- **WHEN** `group_by = { token = "parent" }`, `blocked_first = true`, and agents `one` (workspace 1), `three` and blocked `four` (workspace 2) all report `parent = three`
- **THEN** the `three` header lists `three`, then `four`, then `one`
