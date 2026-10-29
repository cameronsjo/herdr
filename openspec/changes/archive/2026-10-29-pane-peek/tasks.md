## 1. Server

- [x] 1.1 Add a peek to the popup state that borrows a tiled pane's terminal instead of owning one
- [x] 1.2 Add `pane.peek` and `pane.peek_close`, refusing a pane on screen in the current tab and a non-peek popup
- [x] 1.3 Lock the peeked terminal against tab layout resizes, and reapply tab geometry when a peek opens or closes
- [x] 1.4 Close the peek when its pane's terminal shuts down, without skipping the pane's own exit handling
- [x] 1.5 Keep the peeked pane's output live for a client viewing its own tab

## 2. CLI

- [x] 2.1 Add `herdr pane peek <pane_id>|--close`

## 3. Verification

- [x] 3.1 App tests: peek borrows and returns the pane, refuses visible panes, replaces only peeks, closes with its pane
- [x] 3.2 Server test: popup-size while peeked, layout passes leave it alone, layout size restored on close
- [x] 3.3 Pass `scripts/openspec.sh validate --all --strict`

## 4. Docs and ledger

- [x] 4.1 Describe `pane peek` in `cli-reference.mdx` and list the methods in `socket-api.mdx`
- [x] 4.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
