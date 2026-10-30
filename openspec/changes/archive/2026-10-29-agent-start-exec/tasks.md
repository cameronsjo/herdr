## 1. Exec

- [x] 1.1 Add `exec` to `AgentStartParams` and `--exec PATH` to the CLI
- [x] 1.2 Validate it and type it in place of the default executable
- [x] 1.3 Advertise `agent_start_exec` and gate the CLI on it

## 2. Verification

- [x] 2.1 Tests: the typed command and reported argv use exec; invalid exec types nothing; the CLI sends `exec` and refuses a server without the capability
- [x] 2.2 Regenerate the API schema; pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Document `--exec` in `cli-reference.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
