## ADDED Requirements

### Requirement: agent start can launch another executable
`agent.start` SHALL accept an optional `exec`, and `herdr agent start` SHALL accept `--exec PATH`. When it is set, the typed command MUST use it in place of the kind's default executable, quoted like the arguments and not expanded, and the reported `argv` MUST start with it. The kind still selects detection and the managed launch. An empty or whitespace-only `exec`, or one with control characters, MUST fail with `invalid_agent_exec` before anything is typed.

#### Scenario: Wrapper launch
- **WHEN** `herdr agent start w1 --kind claude --exec /opt/bin/my-claude --pane 1 -- --resume` runs in an available sh pane
- **THEN** the pane receives `/opt/bin/my-claude --resume` and the response's `argv` is `["/opt/bin/my-claude", "--resume"]`

#### Scenario: Invalid exec
- **WHEN** `agent.start` is called with `exec: ""`
- **THEN** the response is the error `invalid_agent_exec` and nothing is typed

### Requirement: Servers advertise agent exec support
A server that honors `agent.start`'s `exec` SHALL report `capabilities.agent_start_exec: true` in its ping response and in `herdr status --json`. The CLI MUST NOT send `--exec` to a server that does not report it, and MUST instead fail with `agent_start_exec_unsupported` without starting anything.

#### Scenario: Older server
- **WHEN** `herdr agent start w1 --kind claude --pane 1 --exec my-claude` runs against a server whose capabilities lack `agent_start_exec`
- **THEN** the command fails with `agent_start_exec_unsupported` and no agent is started
