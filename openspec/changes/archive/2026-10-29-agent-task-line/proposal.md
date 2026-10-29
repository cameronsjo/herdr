## Why

In an orchestrator session the sidebar shows each agent's name and state, but not what it is working on, so answering "which worker is on what" means reading every pane (cameronsjo/herdr#98, Linear CAM-439). Pane metadata tokens can already carry a `$summary`-style value, but nothing sets one with a short command and the default layouts don't show it.

## What Changes

- New CLI `herdr agent task <TEXT>|--clear [--target TARGET]`. It sets or clears the pane metadata token `task` with source `herdr:task`. With no `--target` it uses the calling pane (`HERDR_PANE_ID`); `--target` takes an agent name or pane id and resolves it through `agent.get`.
- The default `rows` and `grouped_rows` gain a last `["$task"]` row. An unset token drops its row, so nothing changes until a task is set.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-sidebar`: a new "Agents show a one-line task" requirement.

## Impact

- CLI and config defaults only; no API, protocol or server change. Uses the existing `agent.get` and `pane.report_metadata` methods.
- Files: `src/cli/fork_agent_task.rs` (new), `src/cli.rs`, `src/cli/agent.rs`, `src/cli/agent/bindings.rs`, `src/config/sidebar.rs`, `src/config.rs`, docs in `docs/next/website/src/content/docs/{cli-reference,configuration}.mdx` (and the ja/zh-cn default blocks) and `docs/next/website/src/data/config-reference.json`.
- Sync collision surface: `AgentsSidebarConfig::default()` (upstream owns the `rows` default) and the `agent` subcommand dispatch in `src/cli/agent.rs`.
