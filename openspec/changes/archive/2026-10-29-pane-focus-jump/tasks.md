## 1. CLI

- [x] 1.1 Accept a positional pane id on `pane focus` and send `pane.focus` for it
- [x] 1.2 Keep `--direction` sending `pane.focus_direction`; require one of the two

## 2. Verification

- [x] 2.1 CLI tests: a pane id jumps, a direction moves, neither or both is refused
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Document the jump in `cli-reference.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
