## Purpose
The agent registry covers how the fork names, routes to, and lists agents: pane-less agents registered with a heartbeat TTL, the `agent.type_submit` input primitive, the liveness gate on name-routed input, and the rules that keep agent names and pane aliases trustworthy across restores and moves.

## ADDED Requirements

### Requirement: Pane-less agents register with a heartbeat TTL
The API SHALL provide `agent.register`, `agent.unregister` and `agent.registered`, with CLI `herdr agent register <name>` (options `--source`, `--agent`, `--status`, `--cwd`, repeatable `--token KEY=VALUE`, `--ttl-ms`), `herdr agent unregister <name> [--source ID]` and `herdr agent registered`. A record is keyed by `source` and `name`; registering the same key again MUST replace it and restart its TTL. `agent.list` MUST stay limited to pane agents.

#### Scenario: Register and list
- **WHEN** `herdr agent register reviewer --source cadence.bridge --status working` is run
- **THEN** `agent.registered` returns a `registered_agent_list` containing `reviewer` with status `working` and `ttl_ms` 60000

### Requirement: Unregister reports what it removed
`agent.unregister` SHALL delete the record for its `source` and `name` and report `removed: true` when one existed and `removed: false` otherwise.

#### Scenario: Unregister an unknown record
- **WHEN** `agent.unregister` names a `source` and `name` that are not registered
- **THEN** the response succeeds with `removed: false`

### Requirement: Registration defaults
`ttl_ms` SHALL default to 60000 and MUST be between 1000 and 86400000, `status` SHALL default to `unknown`, and the CLI's default `source` SHALL be `cli`.

#### Scenario: Defaults applied
- **WHEN** `herdr agent register planner` is run with no other options
- **THEN** the record has `source` `cli`, status `unknown` and `ttl_ms` 60000

#### Scenario: TTL out of range
- **WHEN** `agent.register` is called with `ttl_ms: 500`
- **THEN** the response is the error `invalid_params` and nothing is registered

### Requirement: Registered agents expire
A registered record SHALL be removed once its TTL passes without another `agent.register` for the same key, on the server's own timer, with no request needed to trigger it. `agent.registered` MUST NOT return an expired record.

#### Scenario: Heartbeat stops
- **WHEN** an agent registers with `ttl_ms: 1000` and does not register again
- **THEN** after one second it no longer appears in `agent.registered` or in client shells

### Requirement: Registry input is bounded and clean
`agent.register` MUST refuse with `invalid_params` an empty or whitespace `source` or `name`, a `source` containing `/`, any field containing control or format characters, more than 32 tokens, or a field over its byte limit (`source` and `name` 128, `agent` 64, `cwd` 4096, token key 64, token value 256). It MUST refuse a new record with `registered_agent_limit` when 256 are already registered or when the encoded registry would exceed 512 KiB.

#### Scenario: Slash in the source
- **WHEN** `agent.register` is called with `source: "a/b"`
- **THEN** the response is the error `invalid_params`

#### Scenario: Registry full
- **WHEN** 256 records are registered and a 257th distinct key registers
- **THEN** the response is the error `registered_agent_limit`

### Requirement: Registered agents reach clients as an optional endpoint control
The server SHALL send the registry to client shells as the optional endpoint control `fork.registered-agents.v1`, only when its visible contents change, and MUST NOT send it to a client before any agent has ever registered, so existing endpoint message sequences are unchanged. A heartbeat that changes only the deadline MUST NOT trigger a send. A client MUST read an unknown status as `unknown`, drop a malformed record, and clear the list on an unparseable payload.

#### Scenario: No agent ever registered
- **WHEN** a client attaches to a server where no agent has registered
- **THEN** the client receives no `fork.registered-agents.v1` control

#### Scenario: Pure heartbeat
- **WHEN** an agent re-registers with identical fields
- **THEN** no new control is sent to attached clients

### Requirement: agent.type_submit types literal text and presses Enter
The API SHALL provide `agent.type_submit` (CLI `herdr agent type-submit <target> <text>`), which writes `text` into the target agent's pane as typed input followed by Enter, without `agent prompt`'s prompt handling. Empty text MUST be refused with `empty_agent_type_submit`, a disallowed control character with `invalid_agent_type_submit_text`, and a pane with no known running agent with `agent_not_ready`. A blocked agent MUST NOT be refused.

#### Scenario: Slash command into a live agent
- **WHEN** `herdr agent type-submit reviewer /compact` targets a pane running a detected agent
- **THEN** `/compact` is typed into the pane and submitted, and the response is `ok`

#### Scenario: Blocked agent
- **WHEN** the target agent is blocked on a confirmation prompt
- **THEN** the text is still typed and submitted

#### Scenario: Empty text
- **WHEN** `agent.type_submit` is called with `text: ""`
- **THEN** the response is the error `empty_agent_type_submit`

### Requirement: Input never routes by a name with no live agent
`agent.prompt`, `agent.send_keys` and `agent.type_submit` SHALL resolve an agent name only to a pane whose agent is detected, reported by a hook, or still being launched by `agent.start`. When the name exists only on panes with no live agent, the request MUST fail with `agent_not_found` and a message naming every pane holding the name and pointing to `herdr agent explain <pane>`. Read and wait paths (`agent get`, `agent read`, `agent wait`) MUST keep resolving the name.

#### Scenario: Agent has exited
- **WHEN** the pane named `reviewer` has returned to a shell prompt and `agent.prompt` targets `reviewer`
- **THEN** the response is `agent_not_found` and its message names that pane
- **AND** `agent.read` for `reviewer` still resolves the pane

### Requirement: Agent names are validated before sanitizing
Every path that stores an agent name, session restore included, SHALL check the raw name against the agent-name grammar before any label sanitizing, so a name carrying a zero-width character cannot be laundered into another pane's name, and MUST drop a rejected name. On restore, an invalid name and a second pane claiming an already-restored name MUST both be dropped, each logged with the name escaped, then one summary warning.

#### Scenario: Zero-width name in the session file
- **WHEN** a session file stores the agent name `rev\u200biewer`
- **THEN** the restored pane has no agent name and a warning reports a dropped invalid name

#### Scenario: Duplicate stored name
- **WHEN** two panes in a session file both store the agent name `reviewer`
- **THEN** only the first restored pane is named `reviewer`

### Requirement: Cold restore keeps the agent name as a label
When a pane with a stored agent name restores through a fresh shell with no agent, its agent name MUST be dropped so it cannot route input into the shell, and a valid dropped name SHALL become the pane's label when the pane came back with no label of its own. A label the user set MUST win.

#### Scenario: Agent not resumed
- **WHEN** `resume_agents_on_restore = false` and a pane named `planner` with no label restores
- **THEN** the pane has no agent name and its label is `planner`

#### Scenario: Pane already labeled
- **WHEN** the same pane was stored with the label `notes`
- **THEN** its label stays `notes`

### Requirement: Pane id aliases are bounded and die with the pane
When a pane moves and its public id changes, the previous public id SHALL keep resolving to the pane, but only the most recent previous id per pane MUST be kept. Every alias to a pane MUST be removed when that pane closes, including when its tab or workspace closes around it.

#### Scenario: Pane moved twice
- **WHEN** a pane moves from `p_a` to `p_b` and then to `p_c`
- **THEN** `p_b` resolves to the pane and `p_a` no longer does

#### Scenario: Workspace closed after a move
- **WHEN** a moved pane's workspace is closed
- **THEN** none of that pane's previous public ids resolve
