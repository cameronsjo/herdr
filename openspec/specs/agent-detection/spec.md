# agent-detection Specification

## Purpose
The agent detection spec covers the fork's additions to how Herdr identifies which agent runs in a pane, before any screen rule is read.

## Requirements

### Requirement: Extra process names for built-in kinds
The config SHALL accept `[advanced] agent_process_names`, a table from a built-in agent kind to a list of executable names. Process identification MUST treat a process whose executable basename matches one of those names, ignoring case and the suffixes the built-in names ignore, as that kind. Built-in names MUST take precedence. `agent start --kind` and other agent label parsing MUST NOT accept these names. The table SHALL apply at startup and on config reload.

#### Scenario: Wrapper under its own name
- **WHEN** the config has `agent_process_names = { claude = ["my-claude"] }` and a pane's foreground process is `/opt/bin/my-claude`
- **THEN** Herdr identifies the pane's agent as `claude` and reads its screen with the `claude` manifest

#### Scenario: Not a launch kind
- **WHEN** `herdr agent start w1 --kind my-claude --pane <id>` is run with that config
- **THEN** it fails with `unsupported_agent_kind`

### Requirement: Invalid process names are reported and dropped
An entry under an unknown kind, an empty name, a name containing a path separator, a name that already identifies a different built-in kind, and a name listed under two kinds MUST each produce a config warning and MUST NOT change identification; for a name under two kinds the first kind in key order is kept. A built-in name of the same kind is accepted silently.

#### Scenario: Name taken by another kind
- **WHEN** the config has `agent_process_names = { claude = ["codex"] }`
- **THEN** a config warning says `codex` already names `codex`, and `codex` processes are still identified as `codex`
