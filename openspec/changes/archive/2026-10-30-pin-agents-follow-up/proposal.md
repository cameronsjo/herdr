## Why

The first pin change (cameronsjo/herdr#143) left the TUI without a way to pin and dropped every pin on a restart or update, so an orchestrator had to re-pin itself after each one (CAM-497, cameronsjo/herdr#96).

## What Changes

- A palette row `toggle agent pin` (`core:toggle-agent-pin`) and an optional `keys.toggle_agent_pin` binding, unset by default, pin the focused pane's agent or unpin it when it is already pinned.
- `agent.pin` and `agent.unpin` become client-shell methods so the TUI can send them. A server that predates them makes the action report that it is unsupported.
- The session snapshot keeps each pane's `pin_order`. Restore brings the pin back where the agent comes back (a live handoff import or a native agent resume) and drops it where the pane comes back as a fresh shell.
- `herdr agent start --pin --parent ...` keeps a failed parent record as `error.parent_error` when the pin also fails.
- Spec wording: a pinned agent leaves its token group for the pinned run, and the needs-me filter runs before pinning.

## Capabilities

### Modified Capabilities

- `agent-pin`: adds pinning from the TUI, pins surviving restore, and the `--pin --parent` error.
- `agent-sidebar`: adds "A pinned agent leaves its group".

## Impact

- Code: `ToggleAgentPin` in `src/input/keybindings.rs`, `keybind_help.rs`, `src/config/{keybinds,model}.rs`; `toggle_agent_pin` in `src/client/shell/actions.rs`; `PaneSnapshot::pin_order` and three restore sites in `src/persist/restore.rs`; `pin_failure` in `src/cli/agent.rs`.
- API: the two methods' `client_shell` flag in `define_operations!`, and their digests added to `tests/fixtures/endpoint-method-shapes-v1.json`. No existing digest changes.
- Session file: one optional field, so older and newer servers read each other's files.
