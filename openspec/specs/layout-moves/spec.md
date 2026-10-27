# layout-moves Specification

## Purpose
Layout moves let the operator relocate tabs and panes between spaces and reorder spaces, tabs and panes from every surface (keys, palette, CLI, context menus and mouse) with the same outcome, while keeping worktree groups intact.

## Requirements

### Requirement: CLI workspace move
`herdr workspace move <workspace_id> --index N` SHALL send `workspace.move` to reorder a space, and `--index` MUST be a required non-negative integer. `herdr tab move` and the `tab.move` API it calls, including moving a space's last tab and the `same_workspace` refusal, are specified in `workspace-api`.

#### Scenario: Non-numeric index
- **WHEN** `herdr workspace move w_1 --index two` is run
- **THEN** it exits with the usage error `--index must be a non-negative integer` and no request is sent

### Requirement: Every move reachable from keys and palette
The client SHALL provide these bindable actions, each with a palette row and unbound by default: `move_tab_to_space`, `move_tab_to_new_space`, `move_pane_to_space`, `move_pane_to_new_space`, `move_pane_to_new_tab`, `merge_workspace` and `move_workspace_previous|next`. The "to space" and merge actions MUST open a destination picker; the "new space" and "new tab" actions act on the focused tab or pane at once.

#### Scenario: Bound key moves the pane to a new tab
- **WHEN** `[keys] move_pane_to_new_tab` is bound and pressed
- **THEN** `pane.move` is sent for the focused pane with destination `new_tab` and focus

### Requirement: Destination pickers
`move tab to space` and `move pane to space` SHALL open the navigator titled ` Move tab to ` or ` Move pane to `, with a `new space` row first and only the active server's live spaces as destinations. The tab and merge pickers offer spaces only; the pane picker also offers panes, and a pane row MUST open the split-direction chooser rather than guess. Left and right step between spaces. The footer says what accepting the selected row does.

#### Scenario: Pane picker on a space row
- **WHEN** the pane picker's selection is a space row
- **THEN** the footer reads `moves this pane to a new tab in this space`
- **AND** Enter sends `pane.move` with destination `new_tab` in that space

### Requirement: Context menu move items
The tab context menu SHALL offer `Move left`, `Move right`, `Move to space...` and `Move to new space`. The pane context menu SHALL offer `Move to space...`, `Move to new space` and `Move to new tab`. A space's context menu SHALL offer `Move up` and `Move down`, or `Move group up` and `Move group down` when it owns a worktree group; a linked worktree space's menu offers no move items. Each item runs the same action as its keybinding.

#### Scenario: Tab menu
- **WHEN** the operator right-clicks a tab
- **THEN** the menu lists `Move to space...` and choosing it opens the tab destination picker

### Requirement: Mouse moves
Dragging a tab onto another space's sidebar row SHALL move the tab to that space. Dragging from an agent row in the sidebar, or from a pane's top, bottom or left frame off any split handle, SHALL move that pane: a drop on a space row sends it to a new tab there, a drop on another tab opens the split-direction chooser, and a release off every target cancels. An agent-row or pane-frame press released without dragging MUST focus that pane the same way an ordinary click does.

#### Scenario: Tab dropped on a sidebar space
- **WHEN** a tab is dragged from the tab bar and released over another space in the sidebar
- **THEN** `tab.move` is sent with destination type `workspace` for that space

#### Scenario: Pane frame clicked without a drag
- **WHEN** the operator presses and releases on a pane's frame without moving onto a target
- **THEN** `pane.focus` is sent for that pane on release and nothing moves

### Requirement: Pane dropped on a pane
While a pane is dragged, every other pane under the pointer, frame included, SHALL be a drop target, and its outline SHALL be highlighted. A drop on a pane in the same tab sends `pane.swap` with explicit source and target, then `pane.focus` for the dragged pane. A drop on a pane in another tab opens the split-direction chooser against that pane. The dragged pane itself MUST NOT be a target.

#### Scenario: Pane frame dragged onto a pane in the same tab
- **WHEN** the operator presses on `pane_1`'s frame, drags onto `pane_2` in the same tab and releases
- **THEN** `pane.swap` is sent with source `pane_1` and target `pane_2`, followed by `pane.focus` for `pane_1`

#### Scenario: Drag back onto the dragged pane
- **WHEN** a pane drag returns to the dragged pane and is released there
- **THEN** no `pane.swap` is sent

### Requirement: Worktree groups reorder as one block
Space reorder from keys, the palette, the context menu and drag SHALL share one rule: positions count only spaces that are not linked worktrees; a space owning a worktree group moves with its linked worktrees as one block (`workspace.move_block`); a linked worktree space MUST NOT move on its own, and its reorder is a no-op. Reorder wraps at either end. Dropping a group onto one of its own members is a no-op.

#### Scenario: Keyboard reorder of a group owner
- **WHEN** the focused space owns two linked worktree spaces and `move workspace down` runs
- **THEN** one `workspace.move_block` request moves all three spaces together

#### Scenario: Keyboard reorder of a linked worktree
- **WHEN** the focused space is a linked worktree and `move workspace up` runs
- **THEN** no request is sent

### Requirement: Group scope is the owner plus its linked worktrees
A worktree group SHALL consist of the first non-linked space on a repository key plus the linked worktree spaces on that key. A second non-linked space on the same checkout MUST NOT be a group member: it keeps its own sidebar row (visible and droppable even when the group is collapsed), moves on its own, and is neither moved nor closed by a group action. The sidebar, block moves, merge and close dialogs and the server's group close all use this scope.

#### Scenario: Duplicate space on a collapsed group's checkout
- **WHEN** a group is collapsed and a second plain space is open on the same checkout
- **THEN** that space still has its own sidebar row and accepts a dragged tab

### Requirement: Merge picker offers only real targets
The merge picker, titled ` Merge space into `, MUST NOT offer the source space. With no other space it SHALL read `No other space to merge into`, and when a search empties it, `No matching spaces`.

#### Scenario: Only one space
- **WHEN** the operator runs `merge workspace into...` with a single space open
- **THEN** the picker reads `No other space to merge into`
