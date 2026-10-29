## 1. CLI

- [x] 1.1 Add `herdr agent task` with its own parser, carried in the clap tree for help and completion
- [x] 1.2 Resolve `--target` through `agent.get`, default to `HERDR_PANE_ID`

## 2. Sidebar

- [x] 2.1 End the default `rows` and `grouped_rows` with a `$task` row

## 3. Verification

- [x] 3.1 Unit tests: argument parsing, the report it sends, the default rows draw the task only when set
- [x] 3.2 Pass `scripts/openspec.sh validate --all --strict`

## 4. Docs and ledger

- [x] 4.1 Document the command and the new defaults in `docs/next`
- [x] 4.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
