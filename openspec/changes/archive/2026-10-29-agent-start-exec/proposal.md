## Why

A Claude Code wrapper installed under another name can't be launched with `agent start`, because the kind also picks the executable (cameronsjo/herdr#110, Linear CAM-447). This is the second half of the option Cameron chose. The first half, `[advanced] agent_process_names`, lets detection recognize the wrapper.

## What Changes

- `agent.start` takes an optional `exec`. It is typed in place of the kind's default executable, with the same quoting as the arguments. The kind still selects detection and the managed launch.
- An empty `exec`, or one with control characters, fails with `invalid_agent_exec` before anything is typed.
- Servers advertise `capabilities.agent_start_exec`. The CLI's `--exec PATH` refuses an older server with `agent_start_exec_unsupported`, because that server would launch the default executable and report success.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-launch`: two new requirements, one for exec and one for its capability.

## Impact

- API: a new optional field and a new capability. Old requests are unchanged. The schema artifact is regenerated.
- Files: `src/api/schema/{agents,server,tests}.rs`, `src/api/server.rs`, `src/app/agents.rs`, `src/cli/agent.rs`, `src/cli/agent/bindings.rs`, `src/cli/status.rs`, `src/update.rs`, test literals, and `docs/next/api/herdr-api.schema.json` and `cli-reference.mdx`.
- A session restore still resumes with the kind's default executable. `exec` is not persisted.
