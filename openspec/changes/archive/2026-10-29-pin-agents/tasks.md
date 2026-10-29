## 1. Server

- [x] 1.1 Add `TerminalState::pin_order` and `AgentInfo::pin_order`
- [x] 1.2 Add `agent.pin` and `agent.unpin` to `define_operations!` and handle them in `src/app/api/fork_agent_pin.rs`
- [x] 1.3 Carry the pin to client shells as the reserved `herdr:pinned` agent token
- [x] 1.4 Regenerate the API schema

## 2. CLI

- [x] 2.1 Add `herdr agent pin|unpin <TARGET>|--current`
- [x] 2.2 Add `herdr agent start --pin`, pinning after a successful start

## 3. Sidebar

- [x] 3.1 Order pinned agents first in `ordered_agent_pane_ids` and the multi-machine list, keyed `AgentGroupKey::Pinned`
- [x] 3.2 Keep `blocked_first` from moving agents above or within the pinned run
- [x] 3.3 Ignore the reserved token on registered agents

## 4. Tests and ledger

- [x] 4.1 Unit-test pin ordering, the API handler, the CLI parser and the sidebar order and headers
- [x] 4.2 Add the `docs/fork/CHANGES.md` entry with its regression check
