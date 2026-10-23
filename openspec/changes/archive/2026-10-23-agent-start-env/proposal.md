## Why

`herdr agent start` cannot set environment variables for the agent it launches, so an orchestrator that needs one (a Claude worker on a second profile needs `CLAUDE_CONFIG_DIR`) falls back to `herdr pane run "CLAUDE_CONFIG_DIR=... claude ..."`, a separate `agent rename`, and screen-scraping for readiness, losing everything `agent start` gives it (cameronsjo/herdr#103, Linear CAM-228).

## What Changes

- `herdr agent start` takes a repeatable `--env KEY=VALUE` that sets a variable for the agent process only, not for the pane's shell.
- `agent.start` gains an optional `env` object. Omitted or empty, a request behaves exactly as before.
- Keys must match `^[A-Za-z_][A-Za-z0-9_]*$`, a pair without `=` is refused, and values may not contain control characters. No error, log or debug output repeats a value.
- The server advertises `capabilities.agent_start_env`, and the CLI refuses to send `--env` to a server without it, because an older server would drop the field and still report success.

## Capabilities

### New Capabilities

- `agent-launch`: the fork's additions to launching an agent with `agent.start`, starting with per-process environment variables.

### Modified Capabilities

None.

## Impact

- API: `agent.start` params gain `env`; `ServerCapabilities` gains `agent_start_env`; `herdr status --json` shows it. The JSON schema in `docs/next/api/herdr-api.schema.json` is regenerated.
- New fork file: `src/api/schema/fork_agent_env.rs`.
- Sync collision surface (upstream also edits these): `src/api/schema/agents.rs` (`AgentStartParams`), `src/api/schema/server.rs` (`ServerCapabilities`) and every literal of it (`src/api/server.rs`, `src/cli/status.rs`, `src/update.rs`, `src/api/schema/tests.rs`), `src/app/agents.rs` (`start_agent`), `src/cli/agent.rs` and `src/cli/agent/bindings.rs`, `src/platform/{mod,linux,macos,windows,fallback}.rs` (new functions beside `interactive_shell_command`), and upstream's `spec_models_agent_start_target_and_trailing_args` test in `src/cli/spec.rs`, which listed `--env` as a removed legacy option.
