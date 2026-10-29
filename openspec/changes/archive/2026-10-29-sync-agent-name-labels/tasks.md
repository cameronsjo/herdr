## 1. Server

- [x] 1.1 Relabel a linked tab from `agent.start` and `agent.rename`, and carry a pane label that spells the old name
- [x] 1.2 Rename a linked tab's only agent from `tab.rename` when the label is a valid, free agent name
- [x] 1.3 Put the agent name first in the pane border title

## 2. Verification

- [x] 2.1 Tests: linked, auto-named, user-labeled and two-agent tabs; pane label carry; tab rename to a valid, invalid and taken name; border precedence
- [x] 2.2 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Describe the sync in `docs/next/website/src/content/docs/agents.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
