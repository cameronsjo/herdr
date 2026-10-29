## Why

Workers launched as `herdr agent start cos-cadence --kind claude` do not show `cos-cadence` in the agent sidebar (cameronsjo/herdr#97, Linear CAM-434). The `agent` token prefers a reported display name over the herdr agent name, and herdr-bridge reports `claude · <session>` for every Claude pane, so five workers briefed the same way end up with near-identical rows.

## What Changes

- A new agent sidebar token, `name`, shows the herdr agent name and nothing else: no display-name, kind or title fallback. It is omitted for an unnamed agent.
- `agent` is unchanged, so `["name", "agent"]` shows the herdr name beside the reported display name.
- Default rows are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-sidebar`: a new "Agent name token" requirement.

## Impact

- Client and config only; uses the `name` field agents already carry in client snapshots. No API, protocol or server change.
- Files: `src/config/sidebar.rs`, `src/ui/sidebar/tokens.rs`, `src/client/shell/agent_sidebar.rs`, docs in `docs/next/website/src/content/docs/configuration.mdx`.
- A config that uses `name` fails to load on a herdr without this change, like any unknown token.
- Out of scope, tracked in a separate thread: keeping the tab label in sync with the agent name.
