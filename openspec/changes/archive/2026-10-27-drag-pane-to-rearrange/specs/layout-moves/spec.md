## MODIFIED Requirements

### Requirement: Mouse moves
Dragging a tab onto another space's sidebar row SHALL move the tab to that space. Dragging from an agent row in the sidebar, or from a pane's top, bottom or left frame off any split handle, SHALL move that pane: a drop on a space row sends it to a new tab there, a drop on another tab opens the split-direction chooser, and a release off every target cancels. An agent-row or pane-frame press released without dragging MUST focus that pane the same way an ordinary click does.

#### Scenario: Tab dropped on a sidebar space
- **WHEN** a tab is dragged from the tab bar and released over another space in the sidebar
- **THEN** `tab.move` is sent with destination type `workspace` for that space

#### Scenario: Pane frame clicked without a drag
- **WHEN** the operator presses and releases on a pane's frame without moving onto a target
- **THEN** `pane.focus` is sent for that pane on release and nothing moves

## ADDED Requirements

### Requirement: Pane dropped on a pane
While a pane is dragged, every other pane under the pointer, frame included, SHALL be a drop target, and its outline SHALL be highlighted. A drop on a pane in the same tab sends `pane.swap` with explicit source and target, then `pane.focus` for the dragged pane. A drop on a pane in another tab opens the split-direction chooser against that pane. The dragged pane itself MUST NOT be a target.

#### Scenario: Pane frame dragged onto a pane in the same tab
- **WHEN** the operator presses on `pane_1`'s frame, drags onto `pane_2` in the same tab and releases
- **THEN** `pane.swap` is sent with source `pane_1` and target `pane_2`, followed by `pane.focus` for `pane_1`

#### Scenario: Drag back onto the dragged pane
- **WHEN** a pane drag returns to the dragged pane and is released there
- **THEN** no `pane.swap` is sent
