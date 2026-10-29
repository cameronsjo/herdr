## Why

`agent.prompt` succeeds as soon as the prompt's bytes are written to the pane. A busy agent can hold the prompt with no visible change for a while, so an orchestrator that reads that silence as a lost prompt resends it and the agent gets both copies (cameronsjo/herdr#102, Linear CAM-436). Callers need to tell "sent" from "landed".

## What Changes

- A successful `agent.prompt` result gains `delivered`: `typed`, `queued` or `unknown`.
- After the write, the API server watches the agent for up to 1.5 seconds and stops at the first evidence: a non-working agent that starts working (or blocks) or shows the prompt is `typed`; an agent that was already working and shows the prompt is `queued`; no evidence is `unknown`.
- "Shows the prompt" means the first 32 characters of the prompt's first non-blank line, whitespace collapsed, appear on more rows of the detection region than before the write. First lines under 4 characters are never matched.
- With `--wait`, the window is capped by the caller's timeout and the result carries the same field.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-registry`: new requirement "Prompt results report delivery".

## Impact

- API: optional `delivered` on the `agent_prompted` result, omitted by servers that do not check; an unrecognized value reads as `unknown`. No new parameter, method or capability flag, and no TUI codec change.
- Latency: a prompt with no evidence takes up to 1.5 seconds longer. Evidence usually shows within one or two 100 ms polls.
- Limits: agents that collapse long or multi-line pastes (Claude's "[Pasted text #1 +N lines]") never echo them, so a long prompt to a busy agent reads `unknown`. Per-agent screen rules would close that and need live captures.
- Files: new `src/api/fork_prompt_delivery.rs` and `src/api/schema/fork_prompt_delivery.rs`; `src/api/{mod,wait}.rs`, `src/api/schema{,/response}.rs`, `src/app/api/agents.rs`, `tests/cli/agents.rs`, `docs/next/api/herdr-api.schema.json`, the two agent docs pages.
