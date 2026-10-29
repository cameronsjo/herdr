## ADDED Requirements

### Requirement: Peek a pane from another tab
`pane.peek` (CLI `herdr pane peek <pane_id>`) SHALL show the given pane in the popup overlay over the tab the operator is on, with keyboard, paste and mouse input routed to it as for any popup. The pane MUST stay in its own tab: no layout, tab or space changes. A pane in the current tab MUST be refused with `pane_visible`. When a plugin popup is open the request MUST be refused with `popup_open`; when a peek is already open it is replaced.

#### Scenario: Answer a worker without leaving the orchestrator
- **WHEN** the operator is on tab 1 and `herdr pane peek w1:p2` is run for a pane in tab 2
- **THEN** that pane appears in an overlay titled `peek: <name> (w1:p2)` over tab 1
- **AND** keys typed go to that pane, and tab 2's layout is unchanged

#### Scenario: Pane already on screen
- **WHEN** `herdr pane peek` names a pane in the current tab
- **THEN** it fails with `pane_visible` and no overlay opens

### Requirement: Peek identity
The overlay title SHALL include the peeked pane's public id next to its name, because a caller can set any label. A peek SHALL close if its pane is moved or merged into the tab the overlay is shown over, so a pane never shows twice.

#### Scenario: Pane moved under its overlay
- **WHEN** a peeked pane is moved into the tab its overlay is shown over
- **THEN** the overlay closes and the pane shows in its new place

### Requirement: Peek sizing
While peeked, the pane SHALL be sized to the overlay, and tab layout passes MUST leave its size alone. When the peek closes, the next tab geometry pass SHALL give the pane its layout size back.

#### Scenario: Close restores the layout size
- **WHEN** a peek of a pane is closed
- **THEN** the pane's terminal size matches its place in its own tab again

### Requirement: Closing a peek
`pane.peek_close` (CLI `herdr pane peek --close`) SHALL close an open peek and MUST leave any other popup open, failing with `peek_not_open` when no peek is open. A peek SHALL also close when its pane exits or is closed, and that pane's own exit handling MUST still run.

#### Scenario: Peeked pane exits
- **WHEN** the peeked pane's process exits
- **THEN** the overlay closes and the pane is handled like any other exited pane
