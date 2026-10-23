## ADDED Requirements

### Requirement: Warn when per-agent overrides are ignored while grouped
When `[ui.sidebar.agents] group_by` is anything but `none` and `rows_by_agent` has entries, config load SHALL report a diagnostic that names the ignored agent ids and points to `ui.sidebar.agents.grouped_rows`. The config MUST still load as written, and no diagnostic is reported when `group_by = "none"` or `rows_by_agent` is empty.

#### Scenario: Overrides set while grouped are reported
- **WHEN** `group_by = "workspace"` and `rows_by_agent` has entries for `claude` and `codex`
- **THEN** `herdr config check` reports that `rows_by_agent` is ignored while grouped, naming `claude, codex` and `ui.sidebar.agents.grouped_rows`, and the config is kept

#### Scenario: Ungrouped overrides are not reported
- **WHEN** `group_by = "none"` and `rows_by_agent` has an entry for `claude`
- **THEN** no diagnostic about `rows_by_agent` is reported

#### Scenario: Grouped without overrides is not reported
- **WHEN** `group_by = "workspace"` and `rows_by_agent` is empty
- **THEN** no diagnostic about `rows_by_agent` is reported
