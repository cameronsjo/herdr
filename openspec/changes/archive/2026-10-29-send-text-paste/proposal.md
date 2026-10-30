## Why

A long multi-paragraph `pane send-text` into a Claude Code pane arrived cut off (cameronsjo/herdr#121). herdr's write path delivers the bytes intact; the leading hypothesis is that the agent's input handler reads a large unbracketed burst as typing, so an embedded newline or Esc ends or reshapes the prompt. `pane run` already wraps text in bracketed paste when the pane enables it, but `send-text` has no way to (Linear CAM-432).

## What Changes

- `pane.send_text` takes an optional `paste` boolean, and `herdr pane send-text` a `--paste` flag before the pane id.
- With `paste`, the text is wrapped in bracketed paste when the pane has enabled bracketed-paste mode, and sent raw when it has not. Without it, nothing changes.
- A paste refuses text containing ESC (`invalid_paste_text`), which could otherwise end the paste early and turn the rest into typed keys.
- The field is omitted from the request when unset, so existing callers send the same bytes.

## Capabilities

### New Capabilities

- `pane-input`: fork additions to sending input into a pane.

### Modified Capabilities

None.

## Impact

- API: one optional field on `pane.send_text` (`docs/next/api/herdr-api.schema.json` regenerated). An older server ignores it and sends raw text, which is the behavior the caller had before.
- Files: `src/api/schema/panes.rs`, `src/app/api/panes.rs`, `src/cli/pane.rs`, `src/platform/macos/input_tests.rs`, docs in `docs/next/website/src/content/docs/{cli-reference,socket-api}.mdx`.
- Not settled here: whether bracketed paste fixes the cut-off text in a real Claude Code pane. That needs a live repro, tracked on cameronsjo/herdr#121.
