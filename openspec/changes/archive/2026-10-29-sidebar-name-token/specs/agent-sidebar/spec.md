## ADDED Requirements

### Requirement: Agent name token
Agent sidebar rows SHALL accept a `name` token that shows the agent's herdr name, as set by `herdr agent start <name>` or `agent rename`. It MUST NOT fall back to a reported display name, the agent kind or a title, and MUST be omitted when the agent has no herdr name. The `agent` token and the default rows are unchanged. `name` is distinct from the custom token `$name`.

#### Scenario: Named Claude worker with a reported display name
- **WHEN** a row is `["name", "agent"]` and agent `cos-cadence` has reported display name `claude · feat/x`
- **THEN** the row shows `cos-cadence · claude · feat/x`

#### Scenario: Unnamed agent
- **WHEN** a row is `["name", "agent"]` and the agent has no herdr name
- **THEN** the row shows only the `agent` token
