## ADDED Requirements

### Requirement: Pin and unpin an agent
`agent.pin` and `agent.unpin` SHALL take an agent target (pane id or agent name) and return the agent's `AgentInfo`. Pinning gives the agent a `pin_order` one greater than any agent already pinned; pinning an agent that is already pinned MUST keep its order, and unpinning an agent that is not pinned MUST succeed without change. A target that hosts no agent MUST be refused with `agent_not_found`. The pin belongs to the agent's terminal, so it follows the agent when its pane moves.

#### Scenario: Pins keep their order
- **WHEN** agent A is pinned, then agent B, then A is pinned again
- **THEN** A has `pin_order` 1 and B has `pin_order` 2

#### Scenario: A shell pane
- **WHEN** `agent.pin` targets a pane where no agent is detected
- **THEN** the request fails with `agent_not_found` and nothing is pinned

### Requirement: Pin from the CLI
`herdr agent pin <TARGET>` and `herdr agent unpin <TARGET>` SHALL send `agent.pin` and `agent.unpin`; `--current` in place of the target MUST use `HERDR_PANE_ID` and fail with a usage error when it is unset. `herdr agent start ... --pin` SHALL pin the agent by its pane once the start succeeds; a `--pin` after `--` MUST be passed to the agent. When the start succeeds but the pin fails, the command MUST fail with a message saying the agent started but was not pinned.

#### Scenario: Orchestrator pins itself
- **WHEN** an agent in pane `w1:p2` runs `herdr agent pin --current`
- **THEN** the agent in `w1:p2` is pinned

#### Scenario: Agent argument named like the flag
- **WHEN** `herdr agent start orch --kind claude --pane 1 -- --pin` runs
- **THEN** `--pin` is passed to the agent and the agent is not pinned
