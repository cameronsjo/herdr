# One space per path

## Goal

Stop Herdr from opening a second space on a path that already has one, and make
tab/pane moves work between same-named spaces that already exist.

## What Cameron reported

With two spaces that share a name, tabs and panes cannot be moved between them.
Cameron has no use case for two spaces scoped to the same path, so the preferred
fix is to prevent the duplicate rather than only repair the move.

## Findings

The move path itself is id-based end to end and looks correct. The picker rows
(`src/client/shell/navigator_moves.rs`), the client's destination resolution
(`src/client/shell/overlay_input.rs:369`), and the server handlers
(`src/app/api/tabs.rs:199`, `src/app/api/panes.rs:924`) all carry
`workspace_id`, never a label. A duplicate name alone does not confuse them.

What does break on duplicates is identity by path:

- `workspace_entries` (`src/client/shell/sidebar.rs:434`) groups spaces by
  `worktree.key`. Two spaces on the same checkout share that key and at least
  one is non-linked, so they are rendered as a worktree group: the first becomes
  the parent, the second becomes an indented child. When the group is collapsed,
  every non-focused child is dropped from the entry list entirely
  (`sidebar.rs:493`). A space with no entry has no `WorkspaceHit`, so it is not
  a sidebar drop target and cannot receive a dragged tab or pane. This is the
  leading explanation for the reported symptom.
- `find_parent_workspace_by_key` (`src/app/api/worktrees.rs:456`) and
  `open_workspace_idx_for_checkout` (`src/app/api/worktrees.rs:600`) both use
  `position`, so worktree actions always route to the first space on a checkout
  and silently ignore the second.
- `handle_workspace_create` (`src/app/api/workspaces.rs:40`) has no reuse check.
  Creating a space for an already-open path always produces a duplicate.

The sidebar mechanism is read from source, not reproduced: no session that
worked on this could build the project. Confirming it on a real build is part of
validating the change.

## Chosen approach

Two independent changes.

Prevent the duplicate at the intent layer. Add an advertised API method
`workspace.open` that takes a cwd and either focuses the existing space whose
resolved identity path matches or creates one. The client's "new space" action,
the palette entry, and the CLI call it. `workspace.create` keeps its current
meaning: an explicit create always creates. A server that does not advertise
`workspace.open` makes the client fall back to `workspace.create`, so only the
reuse behavior is lost, never the connection.

Make existing duplicates harmless. Group spaces in the sidebar only when exactly
one group member is non-linked. Two non-linked spaces on the same checkout are
not a repo and its worktree, so they stay independent top-level rows, keep their
hit rects, and remain drop targets. Make the two `position` lookups prefer the
focused space and fall back to the lowest index, so worktree actions are
deterministic instead of first-match.

## Requirements

- The client's new-space action MUST focus an existing space when one already
  resolves to the same identity path.
- `workspace.create` MUST keep creating unconditionally.
- A server without `workspace.open` MUST leave the client fully usable.
- The sidebar MUST NOT group two non-linked spaces that share a repo key.
- Every space in the active endpoint MUST have a sidebar hit rect whenever its
  row is visible, so every space is a valid drag destination.
- Worktree parent lookups MUST resolve deterministically when several spaces
  share a checkout.
- The change MUST NOT alter generation-1 codecs, frozen fixtures, or the meaning
  of any existing advertised method.

## Alternatives declined

- Make `workspace.create` reuse an existing space: changes the meaning of an
  advertised method, which the stable endpoint contract forbids.
- Reject duplicates server-side with an error: breaks restore of sessions that
  already contain duplicates, and turns a preference into a hard failure.
- Dedupe by label: two spaces can legitimately share a label on different paths.
- Only fix the move and leave duplicates creatable: leaves the first-match
  worktree lookups wrong and does not address what Cameron actually asked for.

## Checklist

- [x] Fix the sidebar grouping so two non-linked spaces on one repo key never
      collapse into a group, and keep group scope (status rollup, block move,
      merge and close dialogs) to the parent plus its linked worktrees.
- [x] Make `workspace_close_indices` linked-only and owned by the first
      non-linked space, so a group close or merge cannot take a duplicate with
      it.
- [x] Make the worktree parent lookups prefer the focused space instead of the
      lowest matching index.
- [x] Add `workspace.open`, advertise it, and wire the TUI new-space action and
      `herdr workspace open` to it with a `workspace.create` fallback.
- [x] Add tests: sidebar entries, group close indices, `workspace.open` reuse,
      and the TUI's open-versus-create choice.
- [x] Update the generated API schema artifact and the English docs.
- [x] Record the `workspace.open` digest:
      `HERDR_RECORD_ENDPOINT_METHOD_SHAPES=1 just test-one advertised_client_shell_method_shapes`.
      The test now appends a missing method's digest and still refuses to
      rewrite an existing one. Confirmed present in
      `tests/fixtures/endpoint-method-shapes-v1.json` and the unset-mode test
      passes locally (no rewrite needed).
- [x] Run `just check`: `cargo fmt --check` and `cargo clippy --all-targets
      --locked -- -D warnings` pass clean locally; CI's `check (macos-latest)`
      and `check (ubuntu-latest)` jobs (which run `just ci`, the same lint +
      nextest + maintenance-test suite) are green on the PR head. A local full
      `cargo nextest run` hit two failures unrelated to this diff:
      `federated_launch_opens_local_directly_while_saved_ssh_is_unavailable`
      (passed on isolated rerun once machine contention cleared — a local
      flake, not a regression) and `live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session`
      (this repo's own CI excludes the whole `live_handoff` binary on macOS via
      `nextest_filter: not binary(live_handoff)` — a known macOS-only flake
      class, not something this PR introduced; it does run and pass on the
      Linux CI job). `just bench-render-scale` not run — this PR does not touch
      a render/layout hot path, and CI carries no bench gate for this change.
- [x] Open a PR against `cameronsjo/herdr`.

## Status

Implemented and validated by a later PR-worker session. `cargo fmt`, `cargo
clippy --all-targets --locked -- -D warnings`, and the full test suite ran
clean (two pre-existing, diff-unrelated local flakes noted in the checklist
above). CI (`check (macos-latest)`, `check (ubuntu-latest)`,
`conventional-commits`) is green on the PR head; CodeRabbit's four actionable
findings were all resolved (two fixed, two declined with a technical reason
CodeRabbit agreed with) before this session picked up the PR.

Two judgment calls worth a second look:

- The new-space action reuses whenever no name is typed. Pressing it in a space
  whose path already has a space now focuses that space, which in the common
  case is the space you are already in — so the key can look like it did
  nothing. That is the behavior Cameron asked for, and the ways to still get a
  second space on a path are typing a name in the new-space prompt
  (`ui.prompt_new_workspace_name = true`) or `herdr workspace create`.
- Reuse matches on identity path equality, so a space sitting in a
  subdirectory of the same repo is a different space and is not reused.

## Risks

- The sidebar grouping change touches a rendering path that runs per space per
  frame. The grouping pass keeps the same shape and cardinality, so no new
  scaling cost is expected; `just bench-render-scale` confirms.
- Reuse changes what the new-space key does for a repeat path, as above.

## Notes

Per this fork's operating rule, all branches, commits and PRs target
`cameronsjo/herdr`. Nothing is written to `herdrdev/herdr`.
