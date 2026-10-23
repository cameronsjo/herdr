# agent-launch Specification

## Purpose
Agent launch covers what the fork adds to starting an agent with `agent.start` and `herdr agent start`: environment variables scoped to the agent process, how they are validated without exposing values, and how a caller learns that a server supports them.

## Requirements

### Requirement: agent start sets environment variables for the agent process only
`herdr agent start` SHALL accept a repeatable `--env KEY=VALUE`, and `agent.start` SHALL accept an optional `env` object of string values. Each variable MUST be set for the launched agent process and MUST NOT remain in the pane's shell after the agent exits. Values MUST reach the agent literally, with no expansion by the pane's shell.

#### Scenario: Second Claude profile
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env CLAUDE_CONFIG_DIR=/home/me/.claude-alt` is run against a pane at a bash prompt
- **THEN** the agent process has `CLAUDE_CONFIG_DIR=/home/me/.claude-alt` and the pane's shell does not have it after the agent exits

#### Scenario: Value with shell syntax
- **WHEN** `agent.start` is called with `env: {"A": "$HOME 'x' ;b"}`
- **THEN** the agent sees `A` as exactly `$HOME 'x' ;b`

### Requirement: agent env is optional and stays out of argv
A request without `env`, or with an empty one, MUST behave exactly as before and MUST NOT serialize an `env` field. When a key is repeated on the CLI the last value SHALL win. The response's `argv` MUST NOT include the variables.

#### Scenario: Repeated key
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env MODE=a --env MODE=b` is run
- **THEN** the request carries `env: {"MODE": "b"}` and the response's `argv` is `["claude"]`

### Requirement: agent env input is validated without echoing values
Keys MUST match `^[A-Za-z_][A-Za-z0-9_]*$`. The CLI MUST refuse an `--env` argument with no `=` as a usage error before connecting. `agent.start` MUST refuse an invalid key or a value containing a control character with the error `invalid_agent_env`, before anything is typed into the pane. No error message, log line or debug output SHALL contain an env value, and an invalid key MUST NOT be repeated either.

#### Scenario: Missing equals sign
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env sk-secret` is run
- **THEN** it fails with `--env: agent env entries must be KEY=VALUE` and the output does not contain `sk-secret`

#### Scenario: Invalid key over the API
- **WHEN** `agent.start` is called with `env: {"1BAD": "sk-secret"}`
- **THEN** the response is the error `invalid_agent_env`, nothing is typed into the pane, and the response does not contain `sk-secret`

### Requirement: Servers advertise agent env support
A server that honors `agent.start`'s `env` SHALL report `capabilities.agent_start_env: true` in its ping response and in `herdr status --json`. The CLI MUST NOT send `--env` to a server that does not report it, and MUST instead fail with `agent_start_env_unsupported` without starting anything, because an older server would ignore the field and report success.

#### Scenario: Older server
- **WHEN** `herdr agent start worker --kind claude --pane 1 --env A=1` runs against a server whose capabilities lack `agent_start_env`
- **THEN** the command fails with `agent_start_env_unsupported` and no agent is started
