## 1. TUI

- [x] 1.1 Add `KeybindAction::ToggleAgentPin`, its palette row and the optional `keys.toggle_agent_pin`
- [x] 1.2 Send `agent.pin` or `agent.unpin` for the focused pane, and advertise both as client-shell methods

## 2. Restore

- [x] 2.1 Capture `pin_order` in `PaneSnapshot`
- [x] 2.2 Restore it on a handoff import and a native agent resume; drop it on a fresh shell and an unavailable pane

## 3. CLI

- [x] 3.1 Keep `parent_error` when `agent start --pin` fails to pin

## 4. Tests and ledger

- [x] 4.1 Unit-test the palette toggle, snapshot capture, both restore outcomes and the CLI error
- [x] 4.2 Add the `docs/fork/CHANGES.md` entry with its regression check
