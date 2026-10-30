## 1. CLI

- [x] 1.1 Take `--parent NAME` out of `agent start` options before clap parses them; list it in help
- [x] 1.2 Resolve the parent before starting, record the tokens after the agent is ready

## 2. Sidebar

- [x] 2.1 Lead each token group with the agent it is named after, locally and across machines

## 3. Verification

- [x] 3.1 Unit tests: option splitting and validation, the reports sent; client test: the parent leads its group
- [x] 3.2 Pass `scripts/openspec.sh validate --all --strict`

## 4. Docs and ledger

- [x] 4.1 Document `--parent` and the leader rule in `docs/next`
- [x] 4.2 Add the `docs/fork/CHANGES.md` entry, with its regression check
