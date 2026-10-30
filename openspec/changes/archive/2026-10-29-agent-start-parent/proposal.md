## Why

An orchestrator launches workers across tabs and workspaces, and the sidebar has no record of which agent launched which, so its workers scatter across workspace groups (cameronsjo/herdr#98, Linear CAM-442).

## What Changes

- `herdr agent start <name> ... --parent NAME` checks that `NAME` is a live agent before typing anything. Once the new agent is ready it sets the pane metadata token `parent=NAME` (source `herdr:parent`) on the new pane, and on the parent's pane too unless that pane already has a `parent` token.
- Under `group_by = { token = ... }`, an agent whose name equals its group's value leads the group, ahead of blocked members. With `group_by = { token = "parent" }` an orchestrator heads its workers' group.
- No API or protocol change: the flag is CLI-side and the link is an ordinary metadata token. Collapsing a group is left for a follow-up.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-launch`: new "agent start checks the launching agent first" and "agent start records the launching agent" requirements.
- `agent-sidebar`: a new "The agent a token group is named after leads it" requirement.

## Impact

- Files: `src/cli/fork_agent_parent.rs` (new), `src/cli.rs`, `src/cli/agent.rs`, `src/cli/agent/bindings.rs`, `src/client/shell/{agent_sidebar,aggregate_navigation}.rs`, a test in `src/client/shell/tests/agents_worktrees_notifications.rs`, docs in `docs/next/website/src/content/docs/{cli-reference,configuration}.mdx`.
- Sync collision surface: the start branch of `run_agent_command` and `agent_start` in `src/cli/agent.rs`, the `start` leaf in `bindings.rs`, and the tails of `ordered_agent_pane_ids` and `aggregate_agent_rows`.
