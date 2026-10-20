## Why

The fork's own behavior was recorded only in `docs/fork/CHANGES.md`, a ledger of where the code diverges, and in dated `docs/plans/` files that are history, not a description of what ships. Nothing said, in one place per capability, what the fork's behavior is today. Moving to OpenSpec gives each fork capability a living spec that every change updates as it lands (cameronsjo/herdr#112).

## What Changes

- Add `openspec/` with the spec-driven schema and a fork context (`openspec/config.yaml`).
- Record the shipped behavior of the fork's own capabilities as baseline specs, read from the code and checked against `docs/fork/CHANGES.md`. Upstream behavior is out of scope.
- Install the OpenSpec skills and `/opsx:*` commands for Claude Code (`.claude/`) and Codex (`.agents/`).
- Pin the OpenSpec CLI in `scripts/openspec/` (lockfile with integrity hashes) behind `scripts/openspec.sh`, and run `validate --all --strict` in a fork-only `OpenSpec` workflow, kept out of `ci.yml` so syncs never conflict on it.
- Freeze `docs/plans/`: no new plans after 2026-10-20.
- No change to the binary's behavior.

## Capabilities

### New Capabilities

- `command-palette`: the fork's palette rows, recents, ranking, keywords, direction choosers and destructive confirm.
- `layout-moves`: moving tabs and panes between spaces and reordering spaces from every surface, with worktree groups kept intact.
- `agent-sidebar`: agent panel grouping, ordering and clearing an agent view.
- `workspace-api`: `workspace.open`, `workspace.merge`, the `tab.move` destinations and the defensive id and accept-loop behavior.
- `agent-registry`: pane-less agents, `agent.type_submit`, the live-name gate, and agent names across restore.
- `codex-lifecycle`: the fork's hybrid Codex hook lifecycle.
- `labels`: one sanitizer for every label.
- `server-status`: the three server-not-running states and the fork/upstream handoff guard.
- `agent-skill`: the agent skill the binary emits.
- `fork-maintenance`: the divergence ledger rule and the fork's sync, container-check, release and palette-check scripts.

### Modified Capabilities

None. There were no specs before this change.

## Impact

- New: `openspec/`, `.claude/`, `.agents/skills/openspec-*`, `.github/workflows/openspec.yml`, `scripts/openspec.sh`, `scripts/openspec-localize.sh`, `scripts/openspec/`.
- Changed: `AGENTS.md` (inside its "Fork operating rule" section only) and `docs/fork/CHANGES.md`.
- Sync collision surface: none beyond the fork section of `AGENTS.md`; every other path is fork-only.
