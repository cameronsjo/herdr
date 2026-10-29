## Why

An orchestrator that runs `herdr tab create --label cos-cadence` and `herdr agent start cos-cadence` still sees each pane titled by the agent's own session title, which Claude derives from the first prompt, so five workers briefed the same way look alike. Renaming the agent afterwards leaves the tab label behind. One name should follow one agent (cameronsjo/herdr#97, Linear CAM-437). The sidebar side is the `name` token (CAM-434).

## What Changes

- While a pane's agent name routes to a live agent, its border title is its pane label, else its agent name, ahead of the agent's reported session title. A name an agent left behind falls back to upstream's order.
- `agent.rename` carries a tab label that equals the agent's previous name, when the agent is the tab's only named live agent, and emits `tab_renamed`.
- `agent.rename` also carries a pane label that equals the previous name, which is what cold restore leaves behind.
- A label that differs from the name is the user's and nothing here changes it. `tab.rename` and `agent.start` are unchanged: a tab rename never changes an agent's routing name, and a start never relabels a tab, so a failed launch leaves nothing behind.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `labels`: adds "Tab label that spells the agent name follows a rename", "Pane label that spells the agent name follows a rename" and "A live named agent titles its pane".

## Impact

- Server only, no API schema or protocol change. No advertised method changes meaning; `agent.rename` emits `tab_renamed` when it carries a tab label.
- Files: `src/app/agents.rs`, `src/app/api/tabs.rs` (`set_tab_label` split out of `handle_tab_rename`), `src/terminal/state.rs`, tests in `src/app/api/agents.rs`, docs in `docs/next/website/src/content/docs/agents.mdx`.
- Sync collision surface: `rename_agent_target` in `agents.rs`, `handle_tab_rename` in `tabs.rs`, and `TerminalState::border_label`.
- Changes upstream behavior: upstream's border shows the session title over everything, and its `agent_rename_does_not_replace_the_pane_label` still holds for a label that differs from the name.
