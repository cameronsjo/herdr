## Why

An orchestrator supervising many workers across workspaces needs to answer "which agent is waiting on me" without scanning every row (cameronsjo/herdr#98, Linear CAM-441). `blocked_first` moves blocked agents up but still shows everyone, and the Goto picker's blocked filter is a modal, not the sidebar.

## What Changes

- A "needs me" filter for the Agent panel: only blocked agents are listed, from every workspace and machine. The empty result reads ` no agents need you`.
- The header's order toggle cycles `grouped` → `priority` → `needs me` → `grouped`. "needs me" keeps priority order underneath and is drawn in the accent color.
- A palette row `toggle needs me` (`core:toggle-needs-me`) and an optional `keys.toggle_needs_me` binding, unset by default, switch it on and off.
- Session state only: a config reload keeps it, nothing persists it, and nothing is sent to the server.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-sidebar`: new "Needs me shows only blocked agents" and "Turning needs me on and off" requirements.

## Impact

- Client only; no API, protocol or server change.
- The flag rides `AgentsSidebarConfig` as a `#[serde(skip)]` runtime field, so `ordered_agent_pane_ids` (panel, collapsed list, agent navigation) and `aggregate_agent_rows` (multi-machine list) filter in one place each without changing their signatures.
- Files: `src/config/{sidebar,keybinds,model}.rs`, `src/input/{keybindings,keybind_help}.rs`, `src/main.rs`, `src/client/shell/{agent_sidebar,aggregate_navigation,endpoint_agents,actions,config,mouse}.rs`, tests in `src/client/shell/tests/{palette,agents_worktrees_notifications}.rs`, docs in `docs/next/website/src/content/docs/configuration.mdx` and `config-reference.json`.
- Sync collision surface: the sort-toggle click in `mouse.rs`, the header label in `render_agent_panel_header`, `KeybindAction` and the keys tables beside `clear_agent_view`.
