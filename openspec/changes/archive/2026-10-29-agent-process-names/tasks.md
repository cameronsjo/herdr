## 1. Process names

- [x] 1.1 Add `[advanced] agent_process_names` and validate it into a name-to-kind map with diagnostics
- [x] 1.2 Match the names in `identify_agent` after the built-in names, set at startup and on config reload

## 2. Verification

- [x] 2.1 Tests: names map to their kind, bad entries are reported and dropped, a configured name identifies the kind while label parsing stays built-in, and the config parses with warnings
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Document the key in the config reference and on the agents page
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
