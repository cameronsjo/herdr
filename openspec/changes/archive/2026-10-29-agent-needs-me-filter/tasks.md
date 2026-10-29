## 1. Client

- [x] 1.1 Filter blocked agents in `ordered_agent_pane_ids` and `aggregate_agent_rows`
- [x] 1.2 Cycle the header toggle through `needs me`, and say so when nothing is blocked
- [x] 1.3 Add `ToggleNeedsMe` with a palette row and an optional `keys.toggle_needs_me`
- [x] 1.4 Keep the filter across a config reload

## 2. Verification

- [x] 2.1 Client tests: the header cycle and filter, the empty message, the palette toggle
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Describe the filter in `configuration.mdx` and the key in `config-reference.json`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
