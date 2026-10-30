## ADDED Requirements

### Requirement: Extra process names for built-in kinds
The config SHALL accept `[advanced] agent_process_names`, a table from a built-in agent kind to a list of executable names. Process identification MUST treat a process whose executable basename matches one of those names, ignoring case and the suffixes the built-in names ignore, as that kind. Kind keys MUST be matched ignoring case. Built-in names MUST take precedence. `agent start --kind` and other agent label parsing MUST NOT accept these names. It SHALL apply at startup and on reload.

#### Scenario: Wrapper under its own name
- **WHEN** the config has `agent_process_names = { claude = ["my-claude"] }` and a pane's foreground process is `/opt/bin/my-claude`
- **THEN** Herdr identifies the pane's agent as `claude` and reads its screen with the `claude` manifest

#### Scenario: Not a launch kind
- **WHEN** `herdr agent start w1 --kind my-claude --pane <id>` is run with that config
- **THEN** it fails with `unsupported_agent_kind`

### Requirement: Extra names match where built-in names do
An extra name SHALL also identify a wrapper script run through an interpreter, wherever a built-in name is recovered from the interpreter's arguments. A name longer than 15 bytes MUST also match its first 15 bytes, the process name Linux reports.

#### Scenario: Script run by an interpreter
- **WHEN** the config has `agent_process_names = { claude = ["my-claude"] }` and a pane runs `bash /opt/bin/my-claude`
- **THEN** Herdr identifies the pane's agent as `claude`

#### Scenario: Long name on Linux
- **WHEN** the config has `agent_process_names = { claude = ["claude-wrapper-v2"] }` and Linux reports the process as `claude-wrapper-`
- **THEN** Herdr identifies the pane's agent as `claude`

### Requirement: Invalid process names are reported and dropped
An entry under an unknown kind, an empty name, a name containing a path separator, a shell, runtime or launcher name (such as `zsh`, `node`, `python3` or `sudo`), a name that already identifies a different built-in kind, and a name listed under two kinds MUST each produce a config warning and MUST NOT change identification; for a name under two kinds the first kind in key order is kept. A built-in name of the same kind is accepted silently.

#### Scenario: Shell name refused
- **WHEN** the config has `agent_process_names = { claude = ["zsh"] }`
- **THEN** a config warning says `zsh` is a shell, runtime or launcher, and a pane's `zsh` is not identified as an agent

#### Scenario: Name taken by another kind
- **WHEN** the config has `agent_process_names = { claude = ["codex"] }`
- **THEN** a config warning says `codex` already names `codex`, and `codex` processes are still identified as `codex`
