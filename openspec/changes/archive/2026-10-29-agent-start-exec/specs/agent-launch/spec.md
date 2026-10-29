## ADDED Requirements

### Requirement: agent start can launch another executable
`agent.start` SHALL accept an optional `exec`, and `herdr agent start` SHALL accept `--exec PATH`. It MUST replace the default executable in the typed command and start the reported `argv`; a session started this way MUST NOT be recorded for resume. An `exec` that holds anything but ASCII letters, digits and `_ . / : + -`, starts with anything but a letter, digit, `_`, `.` or `/`, or is a shell builtin such as `eval` MUST fail with `invalid_agent_exec` before anything is typed.

#### Scenario: Wrapper launch
- **WHEN** `herdr agent start w1 --kind claude --exec /opt/bin/my-claude --pane 1 -- --resume` runs in an available sh pane
- **THEN** the pane receives `/opt/bin/my-claude --resume` and the response's `argv` is `["/opt/bin/my-claude", "--resume"]`

#### Scenario: Invalid exec
- **WHEN** `agent.start` is called with `exec: "/opt/my claude/run"`
- **THEN** the response is the error `invalid_agent_exec` and nothing is typed

### Requirement: Servers advertise agent exec support
A server that honors `agent.start`'s `exec` SHALL report `capabilities.agent_start_exec: true` in its ping response and in `herdr status --json`. The CLI MUST NOT send `--exec` to a server that does not report it, and MUST instead fail with `agent_start_exec_unsupported` without starting anything.

#### Scenario: Older server
- **WHEN** `herdr agent start w1 --kind claude --pane 1 --exec my-claude` runs against a server whose capabilities lack `agent_start_exec`
- **THEN** the command fails with `agent_start_exec_unsupported` and no agent is started
