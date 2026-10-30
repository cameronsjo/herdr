## Why

In an orchestrator setup the operator returns to one agent all day, and the agents panel keeps moving it as workers change state (cameronsjo/herdr#96). Pinning keeps that agent on top.

## What Changes

- New API methods `agent.pin` and `agent.unpin` take an agent target and return the agent. `AgentInfo` gains an optional `pin_order`.
- New CLI commands `herdr agent pin <TARGET>|--current` and `herdr agent unpin <TARGET>|--current`, and `herdr agent start ... --pin`, which pins the agent once it is ready.
- The agents panel lists pinned agents first, in the order they were pinned, in every sort and view, under a `pinned` header when grouping is on.
- The pin reaches client shells as a reserved agent token (`herdr:pinned`), so the stable snapshot codec is unchanged.
- Not in this change: a TUI keybinding or palette action, and keeping pins across session restore. Both follow in a later change.

## Capabilities

### New Capabilities

- `agent-pin`: pinning an agent through the API and CLI.

### Modified Capabilities

- `agent-sidebar`: adds the "Pinned agents lead the panel" requirement.

## Impact

- Code: `src/fork_agent_pin.rs`, `src/app/api/fork_agent_pin.rs`, `src/cli/fork_agent_pin.rs` (new, fork-owned); `TerminalState::pin_order`; one token push in `src/server/client_shell.rs`; ordering in `src/client/shell/agent_sidebar.rs` and `aggregate_navigation.rs`.
- API: two methods in `define_operations!` and the regenerated `docs/next/api/herdr-api.schema.json`. Neither is a client-shell method, so the endpoint method-shape fixture is unchanged.
- Sync collision surface: the `define_operations!` table, `AgentInfo`, `TerminalState`, `ordered_agent_pane_ids` and `aggregate_agent_rows` are upstream code with fork lines added.
