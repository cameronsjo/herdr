## Why

A peek can only be closed from the command line or by the pane exiting, so an operator who answered a worker's prompt in the overlay has to go back to a shell to dismiss it (cameronsjo/herdr#95, Linear CAM-438).

## What Changes

- While a peek is open, pressing a configured prefix key closes it. The prefix press and any input after it in the same batch never reach the peeked pane.
- Plugin popups are unchanged: they still receive every key.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pane-peek`: "Closing a peek" gains the prefix key.

## Impact

- Server only. The client already sends every key to a popup, so the server spots the prefix in popup input for a peek.
- Files: `src/server/{headless,pane_input}.rs`, tests in `src/server/headless/tests/fork_peek.rs`, docs in `cli-reference.mdx`.
- Sync collision surface: the `ClientShellPopupInput` arm of `handle_server_event` in `headless.rs`.
