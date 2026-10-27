## Why

Herdr can swap two panes from keys, the palette and the pane context menu, and an agent row in the sidebar can be dragged onto a tab or a space, but nothing lets the operator pick up a pane where it sits and drop it somewhere else in its tab. Herdr is mouse-first, so rearranging panes should be a drag too (Linear CAM-396).

## What Changes

- A left press on a pane's top, bottom or left frame (off any split handle; the right side is the scrollbar lane) can grow into a pane drag. Released in place it still focuses the pane, now on release instead of on press.
- While a pane is dragged, any other pane under the pointer, frame included, is a drop target and is outlined in the tab bar's drop-target colors.
- Dropping on a pane in the same tab sends `pane.swap` with explicit source and target, then focuses the dragged pane. Dropping on a pane in another tab (only reachable from an agent-row drag) opens the split-direction chooser against that pane.
- Frame drags reuse every existing pane-drag drop: a tab opens the split chooser, a space row moves the pane to a new tab there, a release off every target cancels.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layout-moves`: "Mouse moves" gains the frame drag source, and a new "Pane dropped on a pane" requirement covers the swap and its highlight.

## Impact

- Client only; no API, protocol or server change. Uses the existing `pane.swap` and `pane.move` methods.
- Files: `src/client/shell/{mouse,state,composition}.rs`, tests in `src/client/shell/tests/mouse_selection.rs`, docs in `docs/next/website/src/content/docs/concepts.mdx`.
- Sync collision surface: `pane_drop_target_at` and the left-press pane branch in `mouse.rs`, `ClientPaneDropTarget` in `state.rs`, and the overlay pass in `composition.rs`.
- With `pane_borders = "off"` a pane has no frame, so only the agent-row drag reaches it.
