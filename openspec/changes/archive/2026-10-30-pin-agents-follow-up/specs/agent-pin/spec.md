## ADDED Requirements

### Requirement: Pin from the TUI
The palette row `toggle agent pin` and the optional `keys.toggle_agent_pin` binding, unset by default, SHALL send `agent.unpin` for the focused pane when its agent is pinned and `agent.pin` otherwise. `agent.pin` and `agent.unpin` MUST be advertised client-shell methods; a server that does not advertise them makes the action report it is unsupported and sends nothing.

#### Scenario: Toggle twice
- **WHEN** the operator runs `toggle agent pin` on a focused agent, and again once the snapshot shows it pinned
- **THEN** the first run sends `agent.pin` and the second `agent.unpin`, both for that pane

### Requirement: A pin survives restore with its agent
The session snapshot SHALL keep each pane's pin order. Restore MUST bring the pin back only where the agent comes back with the pane: a live handoff import, such as an update, or a native agent resume. A pane restored as a fresh shell MUST NOT be pinned.

#### Scenario: Server restart with agent resume
- **WHEN** a pinned agent's pane is restored by resuming the agent's session
- **THEN** the agent keeps its `pin_order`

#### Scenario: Cold restore
- **WHEN** a pinned agent's pane is restored without resuming the agent
- **THEN** the pane is not pinned

### Requirement: A failed pin keeps a failed parent record
When `herdr agent start --pin --parent ...` starts the agent but the pin fails, the error SHALL still carry a failed `--parent` record as `parent_error`, so neither failure hides the other.

#### Scenario: Pin and parent both fail
- **WHEN** the agent starts but neither the parent record nor the pin succeeds
- **THEN** the error reports the pin failure and carries the parent failure as `parent_error`
