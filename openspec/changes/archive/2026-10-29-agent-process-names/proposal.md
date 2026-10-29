## Why

A Claude Code wrapper installed under another executable name runs the same TUI, so the `claude` manifest's screen rules fit it, but Herdr identifies agents by process name and never recognizes it (cameronsjo/herdr#110, Linear CAM-447). Cameron chose the simplest option from the issue: extra process names for a built-in kind, plus `agent start --exec`. This change is the first half; `--exec` follows separately.

## What Changes

- New config key `[advanced] agent_process_names`, a table from a built-in kind to more executable names, e.g. `{ claude = ["my-claude"] }`.
- Process identification also matches those names, on the executable's basename and with the built-in name normalization, so a matching process gets that kind's manifest.
- Invalid entries are dropped with a config warning: an unknown kind, an empty or path-like name, a name that already belongs to another kind, or a name listed under two kinds.
- `agent start --kind` and other label parsing keep the built-in names only.

## Capabilities

### New Capabilities

- `agent-detection`: the fork's additions to how agents are identified.

### Modified Capabilities

None.

## Impact

- Server-side detection plus config; no API, protocol or client change. Applied at startup and on config reload.
- Files: new `src/detect/fork_process_names.rs`; one-line hooks in `src/detect/mod.rs`, `src/config/model.rs`, `src/config.rs` and `src/app/mod.rs`; `docs/next/website/src/data/config-reference.json`; `docs/next/website/src/content/docs/agents.mdx`.
- No new agent kinds and no config-declared kinds (options 2 and 3 in the issue).
