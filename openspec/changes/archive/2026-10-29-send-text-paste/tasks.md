## 1. API and CLI

- [x] 1.1 Add the optional `paste` field to `pane.send_text`, omitted when false
- [x] 1.2 Wrap the text with the pane's live bracketed-paste mode when `paste` is set, before the canonical-mode admission check
- [x] 1.3 Add `--paste` to `herdr pane send-text`

## 2. Verification

- [x] 2.1 API tests: wraps only when the pane accepts paste, raw without the flag, refuses ESC with the flag
- [x] 2.2 CLI tests: the flag before the pane id sets `paste`; after the text it is text
- [x] 2.3 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Document the flag in `cli-reference.mdx` and the field in `socket-api.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
