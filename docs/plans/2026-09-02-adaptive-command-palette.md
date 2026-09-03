# Adaptive command palette

## Goal

Make the fork-only command palette useful before the user types while preserving full command coverage through search. The empty palette should favor commands Cameron actually uses, avoid redundant entries, and remain concise as plugins add commands.

## Chosen approach

Use an adaptive compact empty-query view. Herdr will remember a bounded move-to-front list of selected commands in client-local state, show available remembered commands first, and fill a maximum of 12 rows from a curated core fallback. Typing any query will search the complete available command set.

Each palette command will carry a stable semantic identifier. Core identifiers will be authored with their help entries; plugin identifiers will use the plugin and entrypoint IDs rather than transient vector indexes or display labels. The client will persist at most eight recent identifiers under Herdr's state directory without changing the server protocol or shared session state.

The palette will remove its own `command palette` action, normalize repeated plugin branding, and add `(action)` or `(pane)` only where normalized plugin labels still collide. Search will continue to cover every other core command and every context- and platform-applicable plugin command.

The curated fallback will prioritize creation, movement, layout, and settings actions that benefit from palette discovery: new workspace, new worktree, open worktree, new tab, both split directions, move pane to space, move pane to new tab, move tab to space, zoom pane, toggle sidebar, and settings.

## Requirements

- The empty-query palette MUST display no more than 12 commands.
- The empty-query palette MUST place available remembered commands before curated fallback commands.
- The client MUST remember at most eight distinct selected commands using move-to-front ordering.
- Remembered commands MUST survive a client restart through a client-local state file.
- Missing, malformed, stale, or unwritable history MUST NOT prevent the palette from opening or a command from running; Herdr MUST log persistence failures with the affected path and operation.
- Core, plugin-action, and plugin-pane history MUST use stable semantic identifiers rather than display labels or transient list indexes.
- Typing a non-empty query MUST search every available command except the palette's self-opening command.
- The palette MUST NOT present its own `command palette` action.
- Plugin labels MUST avoid repeating the plugin name when the plugin-authored title already starts with that name.
- Exact normalized collisions between plugin actions and panes MUST be distinguished with `(action)` and `(pane)` suffixes.
- Existing name and keyword match precedence MUST remain unchanged.
- The change MUST remain in the TUI/client presentation layer and MUST NOT alter the Herdr wire protocol.

## Alternatives declined

- Reorder the complete flat list by recent use: this remembers behavior but leaves an empty palette crowded by roughly 70 commands on Cameron's current installation.
- Replace related commands with hierarchical family pickers: this achieves stronger condensation but adds an interaction step and new modal state for commands that search already distinguishes well.
- Rank by permanent execution counts: old habits would become difficult to displace, while a bounded move-to-front list naturally ages stale commands out.
- Deduplicate plugin rows solely by display label: an action and pane can share a title while invoking meaningfully different behavior.

## Checklist

- [x] Inventory the core and currently installed plugin command surface.
- [x] Add stable command identifiers and compact empty-query selection as pure, tested palette logic.
- [x] Add bounded client-local history loading, recording, and failure logging.
- [x] Normalize plugin labels and distinguish exact action/pane collisions.
- [x] Preserve full-query search, keyboard selection, scrolling, and mouse execution behavior with focused tests.
- [x] Update unreleased user documentation for the adaptive palette behavior.
- [x] Run focused diagnostics and tests, then run the repository-supported `just check` equivalent.
- [x] Record implementation deviations and final verification evidence in this plan.

## Implementation notes

Stable core identifiers live on `NavigateAction` rather than individual help entries. This keeps one identifier per semantic action while a test requires every help action except the intentionally omitted self-opening action to resolve to an identifier.

Native Rust builds remain blocked on this Mac by the repository's documented Zig 0.15.2/macOS SDK incompatibility. The supported `./scripts/docker-check.sh` path replaced `just check` without reducing coverage.

## Verification

- Focused palette run: the Docker command `cargo nextest run --locked palette --no-fail-fast` passed all 47 matching tests.
- Full repository run: `./scripts/docker-check.sh` passed formatting, Clippy, 3,649 Rust tests, its 14 documented container-environment exceptions, and all 95 maintenance tests.
- Primary language-server diagnostics reported zero errors across all changed Rust files.
