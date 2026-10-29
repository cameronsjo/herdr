## Why

An orchestrating agent reports sessions by pane id, and reaching one from the CLI takes `workspace focus` and then `tab focus`, because `pane focus` only moves to a neighboring pane (cameronsjo/herdr#94, Linear CAM-433). The `pane.focus` API already switches workspace and tab before focusing; the CLI never exposed it.

## What Changes

- `herdr pane focus <PANE_ID>` sends `pane.focus` for that pane, which switches to its workspace and tab and focuses it.
- `herdr pane focus --direction ...` keeps sending `pane.focus_direction`, unchanged.
- One of the two is required; a pane id cannot be combined with `--direction`, `--pane` or `--current`.

## Capabilities

### New Capabilities

- `pane-focus`: how the fork's CLI focuses a pane.

### Modified Capabilities

None.

## Impact

- CLI only; uses the existing `pane.focus` method, so no API, protocol or server change.
- Files: `src/cli/pane.rs`, `src/cli/pane/bindings.rs`, `src/cli/spec.rs`, docs in `docs/next/website/src/content/docs/cli-reference.mdx`.
- Not covered: the `herdr://` URL scheme and the `url` fields from #94, which need a macOS URL handler.
