## Why

While the agent sidebar is grouped, `grouped_rows` wins over every `rows_by_agent` override, so an override line does nothing and nothing says so (cameronsjo/herdr#31, follow-up to #23). A `grouped_rows_by_agent` table was declined for doubling the config surface over one token; a warning addresses the actual harm, a config line that fails silently.

## What Changes

- Config load reports a diagnostic when `[ui.sidebar.agents] group_by` groups entries and `rows_by_agent` has entries. It names the ignored agent ids and points to `ui.sidebar.agents.grouped_rows`.
- The diagnostic is a warning: the config is kept as written, and the banner does not say "using defaults" or "keeping current config".
- No change to how rows render.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-sidebar`: adds the "Warn when per-agent overrides are ignored while grouped" requirement beside "Grouped rows win over per-agent overrides".

## Impact

- Code: `src/config/sidebar.rs` (new `AgentsSidebarConfig::grouped_rows_by_agent_diagnostic`, tests), `src/config.rs` (one line in `Config::collect_diagnostics`).
- Docs: one sentence in `docs/next/website/src/content/docs/configuration.mdx`.
- Sync collision surface: `Config::collect_diagnostics` in `src/config.rs` is upstream code; the fork adds one `.chain(...)` line. `src/config/sidebar.rs` is fork-owned (`rows_by_agent`/`group_by` are fork config), and `configuration.mdx` is shared docs.
