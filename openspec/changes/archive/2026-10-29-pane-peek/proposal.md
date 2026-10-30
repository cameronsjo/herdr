## Why

An orchestrating agent manages workers in other tabs and spaces, and a worker often needs a quick answer from the operator: a folder-trust dialog, a question, a y/N. Today the operator has to leave the orchestrator's tab to answer, or move the worker's pane into the current tab and back again, which rearranges two layouts (cameronsjo/herdr#95, Linear CAM-438).

## What Changes

- New `pane.peek` API method and `herdr pane peek <pane_id>` command: show a pane from another tab or space in the popup overlay over the current tab, with keys and mouse routed to it. The pane stays in its own tab; only the view is borrowed.
- New `pane.peek_close` method and `herdr pane peek --close`: close the peek and leave any other popup alone.
- While peeked, the pane is sized to the overlay and the tab layout leaves its size alone; closing the peek gives it back its layout size.
- A peek closes on its own when its pane exits or is closed. Peeking a pane replaces an open peek; a plugin popup blocks it; a pane already on screen in the current tab is refused.

## Capabilities

### New Capabilities

- `pane-peek`: showing a pane from elsewhere as a temporary overlay.

### Modified Capabilities

None.

## Impact

- Server and CLI; no client or wire codec change. The peek reuses the popup surface, popup input routing and popup mouse capture the client already has.
- New API methods `pane.peek` and `pane.peek_close`, not admitted to the client shell endpoint, so the frozen endpoint method shapes are untouched.
- Files: `src/app/{popup,state,runtime,api}.rs`, `src/app/api/panes.rs`, `src/api/schema.rs`, `src/cli/pane{,/bindings}.rs`, `src/server/{client_shell,headless/client_views,headless/render}.rs`, `src/ui/panes.rs`, tests in `src/server/headless/tests/fork_peek.rs`.
- Sync collision surface: `PopupPaneState` and `close_popup_pane`, the popup branch of `PaneDied` handling, `any_shell_surface_contains_pane`, and the four tab-resize lock checks in `src/ui/panes.rs`.
