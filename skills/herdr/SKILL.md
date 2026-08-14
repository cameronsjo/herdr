---
name: herdr
description: "Use when the user names Herdr, or asks to inspect or control Herdr panes, tabs, workspaces, commands, or another agent. Requires HERDR_ENV=1. NOT for background terminals, delegation, or parallel work that merely could use one."
---

# Herdr

Herdr organizes terminals into workspaces, tabs, and panes, recognizes coding agents inside them, and exposes the session through the `herdr` CLI.

## Gate

Before any control command, verify this agent is running inside a Herdr-managed pane:

```bash
test "${HERDR_ENV:-}" = 1
```

If the check fails, say you are not running inside Herdr and stop. Do not inspect or control a Herdr session from outside Herdr.

## Safety rules

Read these before issuing commands.

- Use `--no-focus` for background work unless the user asked to switch context.
- Target with `--current`, an explicit pane ID, or a unique agent name. Never rely on another client's focused pane.
- Parse IDs from JSON responses. Never derive them from sidebar order or from examples.
- Do not close workspaces, tabs, panes, or sessions you did not create unless the user asked.
- Never run `herdr server stop` from an active session unless the user intends to stop the server and its pane processes.
- Never kill the main Herdr process. Use a named test session for experiments needing an isolated server.

## Pick the surface

The predicate: does the target pane host a recognized coding agent?

| The pane… | Use | Because |
|---|---|---|
| holds a recognized agent you want to prompt or wait on | `herdr agent …` | Herdr validates agent identity and interprets lifecycle state |
| holds any other process — shell, test, server, editor | `herdr pane …` | Raw terminal control, no agent semantics |
| does not exist yet | `herdr pane split` | `agent start` never creates, splits, or moves layout |

`agent start` requires an existing pane already at its interactive prompt with no foreground command.

## Caller context

Herdr injects the calling pane's location:

```bash
printf '%s\n' "$HERDR_WORKSPACE_ID" "$HERDR_TAB_ID" "$HERDR_PANE_ID"
```

Default to a sibling pane in the current tab and the current working directory. Do not create a workspace, tab, worktree, or different cwd unless the user asks for it.

Inspect geometry with `herdr pane layout --pane "$HERDR_PANE_ID"`, then split a wide pane right and a narrow or tall pane down:

```bash
herdr pane split --current --direction right --cwd "$PWD" --no-focus
```

Avoid repeated same-direction splits, which leave unusably narrow panes. Read the new pane ID from `.result.pane.pane_id`.

## Discover live state

```bash
herdr workspace list
herdr tab list --workspace "$HERDR_WORKSPACE_ID"
herdr pane current --current
herdr pane list --workspace "$HERDR_WORKSPACE_ID"
herdr agent list
```

Server errors are JSON on stderr, exit status 1. Syntax errors exit status 2.

## References

- `references/cli-semantics.md` — command discovery, opaque IDs, ID churn after `pane move`, JSON result paths
- `references/agent-lifecycle.md` — lifecycle states, `--wait` vs `--until`, `agent_prompt_stalled`, `send-keys`
- `references/reading-output.md` — read sources, the alternate-screen ceiling, the file fallback
