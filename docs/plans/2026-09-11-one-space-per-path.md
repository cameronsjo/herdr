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

This session could not build the project (no `zig`, no `just` in the container),
so the sidebar mechanism is read from source, not reproduced. Step 1 confirms it
before any code changes.

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

- [ ] Reproduce: open two spaces on one checkout, collapse the group, confirm
      the second space has no sidebar row and no drop target. Record evidence.
- [ ] Fix `workspace_entries` grouping and the two `position` lookups; add tests
      covering two non-linked same-key spaces.
- [ ] Add `workspace.open`, advertise it, and wire the client new-space action,
      palette entry, and CLI to it with a `workspace.create` fallback.
- [ ] Add API tests: reuse focuses the existing space, `workspace.create` still
      creates, and an unadvertised method degrades to create.
- [ ] Run `just check` and `just bench-render-scale`.
- [ ] Open a PR against `cameronsjo/herdr`.

## Risks

- The sidebar grouping change touches a rendering path that runs per space per
  frame; the grouping pass stays the same shape, so no new scaling cost is
  expected. `just bench-render-scale` confirms.
- Reuse changes what the new-space key does for a repeat path. It focuses
  instead of creating, which is the requested behavior but is a visible habit
  change.

## Notes

Per this fork's operating rule, all branches, commits and PRs target
`cameronsjo/herdr`. Nothing is written to `herdrdev/herdr`.
