## ADDED Requirements

### Requirement: Agents show a one-line task
`herdr agent task <TEXT>` SHALL set the pane metadata token `task` to TEXT with source `herdr:task`, and `herdr agent task --clear` SHALL clear it. Without `--target` it MUST act on the calling pane named by `HERDR_PANE_ID`, and fail with usage when that is unset; `--target` takes an agent name or pane id, resolved through `agent.get`. The default `rows` and `grouped_rows` SHALL end with a `["$task"]` row, which, like any row whose tokens are all unset, is not drawn until a task is set.

#### Scenario: An agent reports its own task
- **WHEN** an agent in pane `p_1_2` runs `herdr agent task "fixing homelab#1093"` with the default rows
- **THEN** `pane.report_metadata` is sent for `p_1_2` with tokens `{"task": "fixing homelab#1093"}`, and the agent's sidebar entry shows that text on a line under its name

#### Scenario: No task set
- **WHEN** an agent has no `task` token and the default rows are in use
- **THEN** its sidebar entry draws no task line

#### Scenario: Clear a worker's task
- **WHEN** an orchestrator runs `herdr agent task --clear --target worker-1`
- **THEN** the CLI resolves `worker-1` to its pane with `agent.get` and sends `pane.report_metadata` with tokens `{"task": null}`
