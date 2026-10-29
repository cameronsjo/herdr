## ADDED Requirements

### Requirement: agent start checks the launching agent first
`herdr agent start` SHALL accept `--parent NAME` (also `--parent=NAME`) among its options before `--`, where NAME matches `[a-z][a-z0-9_-]{0,31}`. It MUST resolve NAME with `agent.get` before starting, and fail without typing into the pane when NAME is not a live agent. `agent.start` itself is unchanged.

#### Scenario: Unknown parent
- **WHEN** `--parent ghost` names no live agent
- **THEN** the command prints the `agent.get` error and starts nothing

### Requirement: agent start records the launching agent
After an agent started with `--parent NAME` is ready, the CLI SHALL set the pane metadata token `parent=NAME`, source `herdr:parent`, on the new agent's pane, and on the parent's pane when that pane has no `parent` token yet.

#### Scenario: Worker started under an orchestrator
- **WHEN** `herdr agent start worker-1 --kind claude --pane p_2_1 --parent orch` succeeds and `orch` has no `parent` token
- **THEN** both `worker-1`'s pane and `orch`'s pane report token `parent = orch`
