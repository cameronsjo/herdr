## Why

An orchestrator that runs `herdr tab create --label cos-cadence` and `herdr agent start cos-cadence` still sees each pane titled by the agent's own session title, which Claude derives from the first prompt, so five workers briefed the same way look alike. Renaming the agent afterwards leaves the tab label behind. One name should follow one agent (cameronsjo/herdr#97, Linear CAM-437). The sidebar side is the `name` token (CAM-434).

## What Changes

- A pane's border title is its agent name when it has one, ahead of the agent's session title, its pane label and its agent kind.
- When a tab holds exactly one named agent, the tab label follows the name while the tab is linked: auto-named, or labeled with the agent's current (for `agent.rename`, previous) name. `agent.start` and `agent.rename` relabel a linked tab and emit `tab_renamed`.
- `tab.rename` on a linked tab also renames the agent when the new label passes the agent-name grammar and no other agent holds it. Otherwise only the tab is renamed, as before.
- `agent.rename` also carries a pane label that spells the agent's previous name, which is what cold restore leaves behind.
- A label that differs from the agent name is the user's and nothing here changes it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `labels`: adds "Tab label follows its only agent's name", "Renaming a linked tab renames its agent", "Pane label that spells the agent name follows a rename" and "Agent name titles the pane".

## Impact

- Server only, no API schema or protocol change. `tab.rename` keeps its response; `tab_renamed` and `pane_updated` events fire for the extra relabels.
- Files: `src/app/agents.rs`, `src/app/api/tabs.rs`, `src/terminal/state.rs`, tests in `src/app/api/agents.rs`, docs in `docs/next/website/src/content/docs/agents.mdx`.
- Sync collision surface: `rename_agent_target` and `start_agent` in `agents.rs`, `handle_tab_rename` in `tabs.rs`, and `TerminalState::border_label`.
- Changes upstream behavior: upstream's border shows the session title over everything, and its `agent_rename_does_not_replace_the_pane_label` still holds for a label that differs from the name.
