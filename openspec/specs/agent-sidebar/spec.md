# agent-sidebar Specification

## Purpose
The agent sidebar panel lists every agent pane; the fork lets the operator group those rows under shared headers, keep blocked agents on top, align trailing tokens, and clear a plugin-set agent view from the TUI.

## Requirements

### Requirement: Group agent rows under one header per workspace
With `[ui.sidebar.agents] group_by = "workspace"` and the panel in `spaces` order, contiguous agents of one workspace SHALL share a single header row showing the workspace label, with no row gap inside the run and `row_gap` between runs. The default is `group_by = "none"`, which draws one self-describing entry per agent with no headers. Values other than `none`, `workspace` or `{ token = NAME }` MUST be rejected when config loads. The `priority` order never shows headers.

#### Scenario: Two agents in one workspace
- **WHEN** `group_by = "workspace"` and two agents run in workspace `api`
- **THEN** the panel shows one `api` header with both agents indented beneath it

#### Scenario: Priority order
- **WHEN** the operator toggles the panel to `priority`
- **THEN** no group headers are drawn

### Requirement: Headers never label another group's agents
A header MUST only be drawn when each group occupies one contiguous run of the order being rendered. When an active agent view's order interleaves workspaces, workspace grouping SHALL turn off for that frame and every agent draws ungrouped. In the multi-machine endpoint agent list, runs are keyed by machine as well as workspace, so two machines advertising the same workspace id get separate headers and a gap between them.

#### Scenario: Interleaving view
- **WHEN** a view orders agents A (workspace 1), B (workspace 2), C (workspace 1)
- **THEN** no headers are drawn

#### Scenario: Same workspace id on two machines
- **WHEN** two endpoints each report workspace `w_1` with agents
- **THEN** the endpoint list shows a header for each machine's run

### Requirement: Group by a pane metadata token
`group_by = { token = "project" }` (a leading `$` is stripped; letters, digits, `_` or `-`, at most 32) SHALL key each agent by the pane's non-empty value for that token, else the same token on its workspace, else its workspace. Token groups span workspaces: each group's members are gathered behind its first member, keeping their relative order, under a header showing the token value. A token value MUST NOT merge with a workspace of the same name.

#### Scenario: Agents in two workspaces share a project
- **WHEN** panes in workspaces 1 and 3 both report `project = herdr`
- **THEN** one `herdr` header lists both agents together

#### Scenario: Workspace token fallback
- **WHEN** a pane has no `project` token and its workspace reports `project = docs`
- **THEN** the agent appears under the `docs` header

### Requirement: Grouped rows win over per-agent overrides
While grouping is in effect each agent entry SHALL render with `grouped_rows` (default: state icon, tab, agent), even when `rows_by_agent` has an entry for that agent, so a header's workspace is not repeated. When not grouped, a `rows_by_agent` entry MUST win over `rows`.

#### Scenario: Claude override while grouped
- **WHEN** `group_by = "workspace"`, `grouped_rows = [["agent"]]` and `rows_by_agent.claude = [["workspace","agent"]]`
- **THEN** a Claude agent's row shows only the agent token

### Requirement: Blocked agents lead
With `[ui.sidebar.agents] blocked_first = true` and the panel in `spaces` order, blocked agents SHALL move to the front of each group while groups are drawn, and to the front of the whole list when they are not, keeping every other relative order. It defaults to `false` and has no effect in `priority` order.

#### Scenario: Blocked agent in a group
- **WHEN** a group lists agents idle, working, blocked in that order and `blocked_first = true`
- **THEN** the blocked agent is drawn first under the header

### Requirement: Header is not painted as the focused row
A group header SHALL never carry the active-row highlight, even when the run's first agent, which draws the header, is focused. A click on the header row MUST focus that run's first agent.

#### Scenario: First agent focused
- **WHEN** the first agent of a grouped run is focused
- **THEN** only its agent row is highlighted and the header row is not

### Requirement: Trailing token groups align right
A sidebar row token written as a table with `align = "right"` SHALL open a trailing group: that token and every visible token after it are drawn against the row's right edge, separators included, and are funded to their full width before other tokens grow. A row with no right-aligned token MUST stay left-packed with no padding. `align` accepts `left` (default) or `right`.

#### Scenario: Right-aligned state text
- **WHEN** a row is `[ "agent", { token = "state_text", align = "right" }, "tab" ]` with room to spare
- **THEN** the row starts with the agent name and ends with `<state> · <tab>` at the right edge

### Requirement: Clear an agent view from the TUI
While an agent view is active the panel header SHALL show the view label followed by ` ✕` in place of the sort toggle, truncating the label so the mark stays visible; clicking it MUST send `agent.view.clear`. The global menu lists `clear agent view` only while a view is active, and the palette row and optional `[keys] clear_agent_view` binding run the same action. View labels have control and format characters (bidi, zero-width) removed.

#### Scenario: Click the mark
- **WHEN** a plugin has set a view labelled `orchestrators` and the operator clicks `orchestrators ✕`
- **THEN** `agent.view.clear` is sent and the header shows the sort toggle again once the view clears

#### Scenario: No active view
- **WHEN** no agent view is active and the global menu opens
- **THEN** it has no `clear agent view` item

### Requirement: Global menu closes when its rows change
When a snapshot arrives that would change the global menu's list of actions (for example an `update ready` row or the `clear agent view` row appearing or disappearing) while the menu is open, the client SHALL close the menu, so a click is never dispatched to a row other than the one drawn. A label-only change that moves no row MUST NOT close it.

#### Scenario: Update lands under an open menu
- **WHEN** the global menu is open and a snapshot adds the `update ready` row
- **THEN** the menu closes before any click is dispatched

### Requirement: Warn when per-agent overrides are ignored while grouped
When `[ui.sidebar.agents] group_by` is anything but `none` and `rows_by_agent` has entries, config load SHALL report a diagnostic that names the ignored agent ids and points to `ui.sidebar.agents.grouped_rows`. The config MUST still load as written, and no diagnostic is reported when `group_by = "none"` or `rows_by_agent` is empty.

#### Scenario: Overrides set while grouped are reported
- **WHEN** `group_by = "workspace"` and `rows_by_agent` has entries for `claude` and `codex`
- **THEN** `herdr config check` reports that `rows_by_agent` is ignored while grouped, naming `claude, codex` and `ui.sidebar.agents.grouped_rows`, and the config is kept

#### Scenario: Ungrouped overrides are not reported
- **WHEN** `group_by = "none"` and `rows_by_agent` has an entry for `claude`
- **THEN** no diagnostic about `rows_by_agent` is reported

#### Scenario: Grouped without overrides is not reported
- **WHEN** `group_by = "workspace"` and `rows_by_agent` is empty
- **THEN** no diagnostic about `rows_by_agent` is reported

### Requirement: Pinned agents lead the panel
The agents panel SHALL list pinned agents before every other agent, in ascending `pin_order`, in `spaces` and `priority` order and under an active agent view (among the agents the view shows). While grouping is in effect the pinned agents MUST form one run under a `pinned` header. `blocked_first` MUST NOT move an agent above or within the pinned run. Agent navigation follows the same order.

#### Scenario: Pinned worker under priority order
- **WHEN** the panel is in `priority` order, agent A is blocked and agent B is pinned
- **THEN** B is drawn first and A second

#### Scenario: Grouped panel
- **WHEN** `group_by = "workspace"` and an agent in workspace `beta` is pinned
- **THEN** the panel shows a `pinned` header with that agent, then the `alpha` and `beta` runs

### Requirement: Pins in the multi-machine list
In the multi-machine agent list, space order SHALL keep each machine's pinned agents at the top of that machine's run, and an order that mixes machines (priority, or a sorting view) SHALL put every pinned agent first, by machine and then `pin_order`.

#### Scenario: Two machines in space order
- **WHEN** machine 2 has a pinned agent and the list is in `spaces` order
- **THEN** that agent leads machine 2's run and machine 1's run is drawn above it unchanged
