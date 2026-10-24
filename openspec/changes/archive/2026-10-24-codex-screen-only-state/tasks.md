## 1. Screen-only Codex state

- [x] 1.1 Re-apply upstream's #5278 state side: reserve `herdr:codex`, drop the codex guards and the visible-working override in `TerminalState`
- [x] 1.2 Stop installing the Codex state hooks, remove them on reinstall, and narrow the hook scripts to `session` and `metadata`
- [x] 1.3 Keep `CODEX_INTEGRATION_VERSION` at 11, since no release ships it yet
- [x] 1.4 Update tests, docs and the ledger entry
- [ ] 1.5 Live check with `codex-state-probe.sh` before merge (cameronsjo/herdr#114)
