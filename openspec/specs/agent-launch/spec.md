# agent-launch Specification

## Purpose
Agent launch covers what the fork adds to starting an agent with `agent.start` and `herdr agent start`: environment variables scoped to the agent process, how they are validated without exposing values, and how a caller learns that a server supports them.

## Requirements

### Requirement: agent start sets environment variables for the agent process only
`herdr agent start` SHALL accept a repeatable `--env KEY=VALUE`, and `agent.start` SHALL accept an optional `env` object of string values. In a pane whose shell is sh, bash, dash, zsh, ksh, mksh, fish or pwsh (and in any pane on Windows), each variable MUST be set for the launched agent process and MUST NOT remain in the pane's shell after the agent exits. Values MUST reach the agent byte for byte, with no expansion by the pane's shell.

#### Scenario: Second Claude profile
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env CLAUDE_CONFIG_DIR=/home/me/.claude-alt` is run against a pane at a bash prompt
- **THEN** the agent process has `CLAUDE_CONFIG_DIR=/home/me/.claude-alt` and the pane's shell does not have it after the agent exits

#### Scenario: Value with shell syntax
- **WHEN** `agent.start` is called with `env: {"A": "$HOME 'x' ;b"}`
- **THEN** the agent sees `A` as exactly `$HOME 'x' ;b`

#### Scenario: Value that breaks naive quoting
- **WHEN** `agent.start` runs in a fish pane with `env: {"A": "\\' true; touch PWN; #"}`
- **THEN** the agent sees `A` as exactly `\' true; touch PWN; #` and no command runs

### Requirement: agent env is refused in shells without verified quoting
When `env` is not empty and the pane's shell is nu, xonsh, elvish, csh, tcsh or any other shell outside the supported list, `agent.start` MUST fail with `agent_env_unsupported_shell` before anything is typed into the pane. The message SHALL name the shell and MUST NOT contain any env value. A request without `env` MUST still start in those shells.

#### Scenario: nu pane
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env A=1` targets a pane at a nu prompt
- **THEN** the error is `agent_env_unsupported_shell`, its message names `nu`, and nothing is typed

### Requirement: a restored agent does not keep its launch env
Variables passed with `env` SHALL NOT be stored in session state. When herdr restores or resumes an agent that was started with `env`, it MUST relaunch it without those variables.

#### Scenario: Resume after restore
- **WHEN** an agent started with `--env CLAUDE_CONFIG_DIR=/alt` is resumed by a session restore
- **THEN** the resumed agent's command line carries no `CLAUDE_CONFIG_DIR`

### Requirement: agent env is optional and stays out of argv
A request without `env`, or with an empty one, MUST behave exactly as before and MUST NOT serialize an `env` field. When a key is repeated on the CLI the last value SHALL win. The response's `argv` MUST NOT include the variables.

#### Scenario: Repeated key
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env MODE=a --env MODE=b` is run
- **THEN** the request carries `env: {"MODE": "b"}` and the response's `argv` is `["claude"]`

### Requirement: agent env input is validated without echoing values
Keys MUST match `^[A-Za-z_][A-Za-z0-9_]*$`. The CLI MUST refuse an `--env` argument with no `=` as a usage error before connecting. `agent.start` MUST refuse an invalid key, or a value containing a control character, U+2028, U+2029 or a format character (such as a bidi override), with the error `invalid_agent_env`, before anything is typed into the pane. No error message, log line or debug output SHALL contain an env value, and an invalid key MUST NOT be repeated either.

#### Scenario: Missing equals sign
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env sk-secret` is run
- **THEN** it fails with `--env: agent env entries must be KEY=VALUE` and the output does not contain `sk-secret`

#### Scenario: Invalid key over the API
- **WHEN** `agent.start` is called with `env: {"1BAD": "sk-secret"}`
- **THEN** the response is the error `invalid_agent_env`, nothing is typed into the pane, and the response does not contain `sk-secret`

### Requirement: a mistyped env is refused without quoting it back
An `agent.start` whose `env` is not an object of string values MUST fail with `invalid_request`, and the error message MUST NOT contain the offending value.

#### Scenario: env sent as a string
- **WHEN** `agent.start` is called with `env: "sk-secret"`
- **THEN** the response is the error `invalid_request` and does not contain `sk-secret`

### Requirement: Servers advertise agent env support
A server that honors `agent.start`'s `env` SHALL report `capabilities.agent_start_env: true` in its ping response and in `herdr status --json`. The CLI MUST NOT send `--env` to a server that does not report it, and MUST instead fail with `agent_start_env_unsupported` without starting anything, because an older server would ignore the field and report success.

#### Scenario: Older server
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env A=1` runs against a server whose capabilities lack `agent_start_env`
- **THEN** the command fails with `agent_start_env_unsupported` and no agent is started

### Requirement: agent start can launch another executable
`agent.start` SHALL accept an optional `exec`, and `herdr agent start` SHALL accept `--exec PATH`. It MUST replace the default executable in the typed command and start the reported `argv`; a session started this way MUST NOT be recorded for resume. An `exec` that holds no `/`, holds anything but ASCII letters, digits and `_ . / : + -`, or starts with anything but a letter, digit, `_`, `.` or `/` MUST fail with `invalid_agent_exec` before anything is typed.

#### Scenario: Wrapper launch
- **WHEN** `herdr agent start w1 --kind claude --exec /opt/bin/my-claude --pane 1 -- --resume` runs in an available sh pane
- **THEN** the pane receives `/opt/bin/my-claude --resume` and the response's `argv` is `["/opt/bin/my-claude", "--resume"]`

#### Scenario: Invalid exec
- **WHEN** `agent.start` is called with `exec: "/opt/my claude/run"`
- **THEN** the response is the error `invalid_agent_exec` and nothing is typed

### Requirement: Servers advertise agent exec support
A server that honors `agent.start`'s `exec` SHALL report `capabilities.agent_start_exec: true` in its ping response and in `herdr status --json`. The CLI MUST NOT send `--exec` to a server that does not report it, and MUST instead fail with `agent_start_exec_unsupported` without starting anything.

#### Scenario: Older server
- **WHEN** `herdr agent start w1 --kind claude --pane 1 --exec /opt/bin/my-claude` runs against a server whose capabilities lack `agent_start_exec`
- **THEN** the command fails with `agent_start_exec_unsupported` and no agent is started
