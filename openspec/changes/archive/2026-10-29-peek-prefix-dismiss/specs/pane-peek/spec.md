## MODIFIED Requirements

### Requirement: Closing a peek
`pane.peek_close` (CLI `herdr pane peek --close`) SHALL close an open peek and MUST leave any other popup open, failing with `peek_not_open` when no peek is open. Pressing a configured prefix key while a peek is open SHALL close it, and that key press and any input after it in the same batch MUST NOT reach the peeked pane; other popups still receive the prefix. A peek SHALL also close when its pane exits or is closed, and that pane's own exit handling MUST still run.

#### Scenario: Peeked pane exits
- **WHEN** the peeked pane's process exits
- **THEN** the overlay closes and the pane is handled like any other exited pane

#### Scenario: Prefix key dismisses the peek
- **WHEN** the operator presses the prefix key while a peek is open
- **THEN** the overlay closes, the key is not sent to the peeked pane, and the pane goes back to its layout size
