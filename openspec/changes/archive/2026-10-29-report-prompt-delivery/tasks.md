## 1. Server

- [x] 1.1 Add `PromptDelivery` and the optional `delivered` field on `agent_prompted`
- [x] 1.2 Capture the agent's status and echo count before the write, then poll status and the detection region after it
- [x] 1.3 Fill `delivered` on both the plain and the `--wait` prompt paths

## 2. Verification

- [x] 2.1 Unit tests for the needle, echo counting, classification and response rewriting
- [x] 2.2 CLI tests: an idle agent's prompt reads `typed`, an already working agent's reads `queued`
- [x] 2.3 Regenerate `docs/next/api/herdr-api.schema.json`
- [x] 2.4 Pass `scripts/openspec.sh validate --all --strict`

## 3. Docs and ledger

- [x] 3.1 Describe `delivered` in `cli-reference.mdx` and `agent-automation.mdx`
- [x] 3.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
