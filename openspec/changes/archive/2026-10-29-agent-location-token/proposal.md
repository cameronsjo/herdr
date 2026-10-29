## Why

Token grouping gathers agents from several workspaces under one header, and the default `grouped_rows` show only the tab, so a grouped orchestrator view does not say where each worker lives (cameronsjo/herdr#98, Linear CAM-440). The `workspace` token would repeat the name on every row, including the agents right in front of the operator.

## What Changes

- New agent sidebar row token `location`. For an agent outside the focused workspace it draws `workspace · tab` (just the workspace when the tab name is hidden, as for `tab`); for an agent in the focused workspace it draws nothing, so its row drops like any row of unset tokens.
- Defaults are unchanged; the docs suggest `["state_icon", "agent", "location"]` for `grouped_rows` under token grouping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-sidebar`: a new "Location names where an agent lives" requirement.

## Impact

- Client only; no API, protocol or server change. Reads the snapshot's `focused_workspace_id`.
- Files: `src/config/sidebar.rs`, `src/ui/sidebar/tokens.rs`, `src/client/shell/agent_sidebar.rs`, a test in `src/client/shell/tests/agents_worktrees_notifications.rs`, docs in `docs/next/website/src/content/docs/{,ja/,zh-cn/}configuration.mdx`.
- Sync collision surface: `AgentSidebarToken` and its name table, `AgentTokenContext`, and the context built in `agent_row`.
