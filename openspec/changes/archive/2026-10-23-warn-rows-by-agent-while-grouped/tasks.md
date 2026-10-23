## 1. Diagnostic

- [x] 1.1 Add `AgentsSidebarConfig::grouped_rows_by_agent_diagnostic` in `src/config/sidebar.rs`
- [x] 1.2 Chain it into `Config::collect_diagnostics` beside the sidebar bounds diagnostic
- [x] 1.3 Keep its wording out of `config_diagnostic_summary`'s "using defaults" and "keeping current config" classes
- [x] 1.4 Unit-test it present when grouped with overrides and absent when ungrouped or without overrides

## 2. Docs and ledger

- [x] 2.1 Add one sentence on the warning to `configuration.mdx`
- [x] 2.2 Add the `docs/fork/CHANGES.md` entry with its regression check
