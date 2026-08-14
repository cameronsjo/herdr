# CLI semantics

## Learn the current CLI

The installed binary is the authority for command syntax. Start with:

```bash
herdr --help
```

Then print the relevant command group by running the group without a subcommand:

```bash
herdr agent
herdr pane
herdr workspace
herdr tab
herdr worktree
herdr terminal
herdr notification
herdr integration
herdr session
```

Do not run bare `herdr` for discovery; it launches or attaches the TUI. Do not probe a mutating nested command by omitting arguments — commands such as `herdr workspace create` are valid with defaults and will execute.

Most control commands return JSON. Read identifiers and state from those responses instead of predicting them.

## Opaque IDs

Public IDs are opaque stable handles:

- workspace: `w1`
- tab: `w1:t1`
- pane: `w1:p1`

Closed tab and pane IDs are not reused.

## ID churn after `pane move`

A pane moved into another workspace receives a new workspace-qualified pane ID. After `pane move`, continue with `.result.move_result.pane.pane_id` or the live agent name.

The old value is reported as `.result.move_result.previous_pane_id`. Only the moved process's inherited caller context keeps resolving that old ID, so do not use it as a general agent target.

## Agent targets

Agent commands accept either a unique live agent name or the pane ID currently hosting that agent. They do not accept terminal IDs or bare agent-kind labels.

Names must match `[a-z][a-z0-9_-]{0,31}` and be unique among live agents. A name follows the current pane occupant and is cleared when that agent exits, is released, or is replaced.

## JSON result paths

Creation responses expose the IDs to use next:

| Command | New IDs at |
|---|---|
| `workspace create` | `.result.workspace`, `.result.tab`, `.result.root_pane` |
| `tab create` | `.result.tab`, `.result.root_pane` |
| `pane split` | `.result.pane` |
| `pane move` | `.result.move_result.pane.pane_id` |
