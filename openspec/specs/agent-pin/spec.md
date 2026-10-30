# agent-pin Specification

## Purpose
Agent pin lets the operator keep chosen agents, such as an orchestrator, at the top of the agents panel through the API and CLI. The panel side is in `agent-sidebar`.

## Requirements

### Requirement: Pin and unpin an agent
`agent.pin` and `agent.unpin` SHALL take an agent target (pane id or agent name) and return the agent's `AgentInfo`. Pinning gives the agent a `pin_order` one greater than any agent already pinned; pinning an agent that is already pinned MUST keep its order, and unpinning an agent that is not pinned MUST succeed without change. A target that hosts no agent MUST be refused with `agent_not_found`.

#### Scenario: Pins keep their order
- **WHEN** agent A is pinned, then agent B, then A is pinned again
- **THEN** A has `pin_order` 1 and B has `pin_order` 2

#### Scenario: A shell pane
- **WHEN** `agent.pin` targets a pane where no agent is detected
- **THEN** the request fails with `agent_not_found` and nothing is pinned

### Requirement: A pin goes with its agent
The pin SHALL belong to the agent's terminal, so it follows the agent when its pane moves, and it MUST be cleared when the agent exits or the terminal respawns. `agent.unpin` on a pane id whose agent is no longer detected MUST still clear that pane's pin.

#### Scenario: Agent exits
- **WHEN** a pinned agent exits and a new agent later starts in the same pane
- **THEN** the new agent is not pinned

#### Scenario: Stale pin
- **WHEN** `agent.unpin` targets the pane of a pinned agent that is no longer detected
- **THEN** the pin is cleared and the request reports `agent_not_found`

### Requirement: Pin from the CLI
`herdr agent pin <TARGET>` and `herdr agent unpin <TARGET>` SHALL send `agent.pin` and `agent.unpin`; `--current` in place of the target MUST use `HERDR_PANE_ID` and fail with a usage error when it is unset. `herdr agent start ... --pin` SHALL pin the agent by its pane once the start succeeds; a `--pin` after `--` MUST be passed to the agent. When the start succeeds but the pin fails, the command MUST fail with a message saying the agent started but was not pinned.

#### Scenario: Orchestrator pins itself
- **WHEN** an agent in pane `w1:p2` runs `herdr agent pin --current`
- **THEN** the agent in `w1:p2` is pinned

#### Scenario: Agent argument named like the flag
- **WHEN** `herdr agent start orch --kind claude --pane 1 -- --pin` runs
- **THEN** `--pin` is passed to the agent and the agent is not pinned

### Requirement: Pin from the TUI
The palette row `toggle agent pin` and the optional `keys.toggle_agent_pin` binding, unset by default, SHALL send `agent.pin` for the focused pane when its agent is listed and not pinned, and `agent.unpin` otherwise, so a pane whose agent is no longer detected can still drop a stale pin. `agent.pin` and `agent.unpin` MUST be advertised client-shell methods; a server that does not advertise them makes the action report it is unsupported and sends nothing.

#### Scenario: Toggle twice
- **WHEN** the operator runs `toggle agent pin` on a focused agent, and again once the snapshot shows it pinned
- **THEN** the first run sends `agent.pin` and the second `agent.unpin`, both for that pane

#### Scenario: Toggle on a pane with no listed agent
- **WHEN** the operator runs `toggle agent pin` on a pane the snapshot lists no agent for
- **THEN** it sends `agent.unpin` for that pane

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
