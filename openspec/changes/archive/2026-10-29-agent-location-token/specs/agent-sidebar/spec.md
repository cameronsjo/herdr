## ADDED Requirements

### Requirement: Location names where an agent lives
The agent row token `location` SHALL draw `<workspace label> · <tab label>` for an agent whose workspace is not the snapshot's focused workspace, or only the workspace label when the tab label is hidden (one tab without a custom label). For an agent in the focused workspace it MUST draw nothing. In the multi-machine endpoint list each machine's own focused workspace decides.

#### Scenario: Agent in another workspace
- **WHEN** rows are `[["agent", "location"]]`, the focused workspace is `client-shell`, and agent `there` runs in workspace `homelab` on the custom-labelled tab `review`
- **THEN** its row reads `there · homelab · review`

#### Scenario: Agent in the focused workspace
- **WHEN** the same rows are used for agent `here` in the focused workspace
- **THEN** its row reads `here`
