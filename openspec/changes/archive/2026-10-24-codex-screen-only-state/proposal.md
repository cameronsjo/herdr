## Why

The fork kept a hybrid Codex lifecycle (state hooks plus screen correction) after upstream dropped Codex's turn hooks and made `herdr:codex` a reserved native-state source (herdrdev/herdr#5278). Reverting that commit cost nineteen conflicted files in the 2026-10-15 sync and leaves a collision surface for every later one. Both sides now ship the same Codex manifest, which reads Codex's own terminal title (`Action Required`, the working spinner) before any screen text, and in the hybrid the screen already wins every disagreement. The hooks also misroute under a shared Codex daemon. Implements cameronsjo/herdr#114.

## What Changes

- Codex state comes from screen detection only, as upstream: `herdr:codex` is reserved again, and the fork's codex arms in `TerminalState` (the visible-working override) are gone.
- `herdr integration install codex` no longer registers the working, blocked and idle hooks, and removes them from an older install. The session hook and the fork's title (`metadata`) hook stay.
- The hook script accepts only `session` and `metadata`.
- `CODEX_INTEGRATION_VERSION` stays 11: no fork release has shipped 11 yet (v0.9.3-palette.1 ships 10). A v11 hook left over from a master build still sends state reports, and the server now discards them.

## Collision surface

Shrinks. `src/agent_resume.rs`, `src/app/actions.rs` and `src/terminal/state.rs` match upstream for Codex again. What is left is the hook scripts (`src/integration/assets/codex/herdr-agent-state.{sh,ps1}`), `src/integration/{mod,targets,tests}.rs` and `tests/cli/hooks.rs`, all for the title hook.
