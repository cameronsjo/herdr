# Fork divergence ledger

Every place `cameronsjo/herdr` (`fork`) has diverged from `herdrdev/herdr`
(`origin`), newest first, grouped by the plan that landed it. Every fork PR
adds its entry to the top of the relevant section (or a new section) when it
merges. Every upstream sync re-runs the regression checks listed below for
every section still in the fork, before the sync merge is pushed.

## Upstream syncs

One line each — these replace no fork behavior, they just pull upstream
forward. Full history: `git log --oneline --merges origin/master..HEAD`.

- (pending) PR — merge upstream through `bc8c289` (4 commits); merged cleanly, no conflicts
- `2b7fec0` PR [#137](https://github.com/cameronsjo/herdr/pull/137) (2026-10-26) — merge upstream through `5608de0` (7 commits); no conflicts, one deletion refused (below)
- `1162b4b` PR [#123](https://github.com/cameronsjo/herdr/pull/123) (2026-10-22) — merge upstream through `5bd900d` (3 commits); merged cleanly, no conflicts
- `ef1489e` PR [#111](https://github.com/cameronsjo/herdr/pull/111) (2026-10-19) — merge upstream through `143bf5f` (4 commits); merged cleanly, no conflicts
- `2cf4d2c` PR [#109](https://github.com/cameronsjo/herdr/pull/109) (2026-10-15) — merge upstream through `916fcf8` (21 commits); conflict record below
- `746b19a` PR [#105](https://github.com/cameronsjo/herdr/pull/105) (2026-10-12) — merge upstream through `dc6b6e5` (16 commits); conflict record below
- `1f576c6` PR [#104](https://github.com/cameronsjo/herdr/pull/104) (2026-10-08) — merge upstream through `4dc23bb` (24 commits); conflict record below
- `704f48a` PR [#93](https://github.com/cameronsjo/herdr/pull/93) (2026-10-05) — merge upstream through `e35f393` (2 commits); merged cleanly, no conflicts
- `97dcb16` PR [#92](https://github.com/cameronsjo/herdr/pull/92) (2026-10-03) — merge upstream through `5da0a01` (29 commits); conflict record below
- `3f910b3` PR [#90](https://github.com/cameronsjo/herdr/pull/90) (2026-09-29) — merge upstream through `d5680d8` (27 commits); conflict record below
- `38b3ac7` PR [#87](https://github.com/cameronsjo/herdr/pull/87) (2026-09-25) — merge upstream through `8d95e9bd` (53 commits); conflict record below
- `7c571fbd` PR [#84](https://github.com/cameronsjo/herdr/pull/84) (2026-09-17) — merge upstream through `101ccc20`
- `e563360d` PR [#57](https://github.com/cameronsjo/herdr/pull/57) (2026-09-08) — merge upstream through `9e01168b`
- `e839cf59` PR [#56](https://github.com/cameronsjo/herdr/pull/56) (2026-09-07) — sync upstream master into the fork
- `c8137c07` PR [#43](https://github.com/cameronsjo/herdr/pull/43) (2026-09-04) — rebuild the fork on upstream master
- `2009bbeb` (2026-08-31) — merge `origin/master` into `sync-upstream-20260831`
- `407c6e85` (2026-08-26) — merge `origin/master` into `sync-upstream-20260826`, cut `v0.8.2-palette.3`
- `8038308d` PR [#22](https://github.com/cameronsjo/herdr/pull/22) (2026-08-23) — sync fork with upstream master (14 commits)
- `26b7baf4` (2026-08-20) — merge `origin/master` into `sync-upstream-20260820`
- `7500ceff` (2026-08-18) — merge `origin/master` into `sync-upstream-20260818`
- `8e36a62f` (2026-08-13) — merge `origin/master` into `sync-upstream-20260813`
- `7beb3323` (2026-08-06) — merge `origin/master` into `sync-upstream-20260806`
- `8a6f4248` (2026-08-05) — merge `origin/master` into `chore/sync-upstream`

### 2026-10-26 sync: conflict resolutions

Nothing conflicted. Upstream's `b796ca6` deletes the `CLAUDE.md` symlink to
`AGENTS.md`, and the fork keeps it: Claude Code reads `CLAUDE.md`, not
`AGENTS.md`, so without the link fork sessions and the sync routine lose the
fork operating rule (never write to `herdrdev/herdr`) and the fork working
notes. Upstream's deletion is now in the merge history, so later syncs carry
the link forward with no further step unless upstream re-adds the file.

- **Collision:** `CLAUDE.md` is a fork-kept symlink upstream deleted. If a
  later sync shows it deleted or replaced, restore it with
  `git checkout HEAD -- CLAUDE.md`.

### 2026-10-15 sync: conflict resolutions

Nineteen files conflicted, in two groups. `skills/herdr/SKILL.md` keeps the
fork's split skill, and `integrations.mdx` keeps the fork's Codex text.

- **Codex turn hooks** (`src/integration/{mod,targets,tests}.rs`,
  `assets/codex/herdr-agent-state.{sh,ps1}`, plus the auto-merged
  `src/agent_resume.rs`, `src/app/actions.rs`, `src/terminal/state.rs` and
  `tests/cli/hooks.rs`): upstream dropped Codex's working/idle hooks and made
  `herdr:codex` a reserved native-state source, so screen detection alone
  drives Codex state (#5278). The fork's hybrid lifecycle (working, blocked,
  idle and metadata hooks with screen correction) is kept: that commit is
  reverted in full, including the files that merged without a conflict,
  because `is_reserved_native_state_source` would otherwise have discarded
  every fork hook report silently. Its
  `install_codex_removes_turn_status_hooks_from_previous_integration` test is
  dropped. Upstream's UTF-8 stdin read for PowerShell hooks (#5270) is applied
  to the fork's Codex script. `CODEX_INTEGRATION_VERSION` goes to 11, because
  upstream's session-only script is now also v10 and would not reinstall.
  **Collision:** any upstream edit to the Codex hook scripts, the codex arms of
  `is_reserved_native_state_source`, or the codex guards in
  `TerminalState`.
- **CLI and API command contracts** (`src/api/{mod,schema,server}.rs`,
  `src/api/schema/{agents,workspaces}.rs`, `src/cli/{agent,completion,spec,tab,workspace}.rs`,
  `src/server/client_commands.rs`; `src/cli/runtime.rs` deleted): upstream
  now declares every API method once in `define_operations!` with its UI and
  client-shell flags, and builds CLI commands from `cli_params!` projections
  (#5212). The fork's methods are added to that table with the flags they had
  before: `workspace.open`, `workspace.merge` and `tab.move_to_destination`
  change the UI and are client-shell methods, `agent.type_submit` changes the
  UI, and the registered-agent methods do neither. The fork's client-shell
  admissions for `agent.view.clear`, `pane.move`, `plugin.action.invoke`,
  `plugin.list` and `plugin.pane.open` are set in the same table, and
  upstream's UI classification test lists the four fork methods. The fork's
  CLI commands moved into upstream's binding files: `workspace open`, `move`
  and `merge` are `workspace_bindings` leaves, `tab move` is a
  `tab_bindings` leaf that still rejects mixed destinations, and
  `agent type-submit` is an agent binding. `agent register`, `unregister` and
  `registered` keep their own parser in `fork_registered_agents.rs` and add
  their clap shape to the agent tree for help and completion. Upstream now
  registers `server live-handoff` as a hidden command; the fork un-hides it in
  `command()` so `herdr server --help` still lists it, and completions use
  `command_for_completions()`, which leaves it out. Upstream's
  `completions_hide_private_commands_arguments_and_values` test reads that
  tree, and checks `--group` on `workspace close` only, because
  `workspace merge --group` is public in the fork. **Collision:** the
  `define_operations!` table, the `LEAVES` arrays, and `agent::bindings::command`.

### 2026-10-12 sync: conflict resolutions

Two files conflicted:

- **Workspace id bounds** (`src/app/ids.rs`): upstream fixed the same
  out-of-range `w_N` / `N` panic as fork PR #99 with the same filter
  (#5171). The fork's line and its comment are kept; upstream's
  `numeric_workspace_ids_beyond_the_last_workspace_are_not_found` test merged
  in alongside the fork's request-path tests.
- **Close pane keybinding** (`src/client/shell/actions.rs`): upstream moved
  `ClosePane` out of the keybinding-to-method table into
  `request_pane_close`, which confirms a workspace's last pane locally
  (#5173). Its arm is dropped from the table; the fork's `MoveTabToNewSpace`,
  `MovePaneToNewSpace` and `MovePaneToNewTab` arms beside it stay. A parent
  group's last pane still goes to the server's `confirmation_required` path,
  because the fork's `open_close_confirmation` declines a tab close that
  would take a group. **Collision:** the action table in
  `endpoint_method_for_action`.

### 2026-10-08 sync: conflict resolutions

Three files conflicted:

- **Release workflow** (`.github/workflows/release.yml`): fork-owned and kept.
  Upstream only bumped action pins there (install-nix-action, rust-cache,
  action-gh-release); the fork's copy keeps its own pins.
- **API accept loop** (`src/api/server.rs`): upstream reworked its
  `run_accept_loop` to spawn threads through the new `crate::thread_spawn`
  (#4958). The fork's `AcceptBackoff` / `SpawnDrops` loop is kept as before,
  and now spawns both the accept thread and each connection thread through
  `crate::thread_spawn::spawn_named`. A failed accept-thread spawn removes the
  bound socket, as upstream does, and the injected-failure test hook covers the
  fork's loop. Upstream's `run_accept_loop`, `spawn_connection_handler` and
  their `accept_loop_tests` are dropped again; upstream's
  `api_accept_thread_spawn_failure_removes_bound_socket` stays.
  **Collision:** the accept-thread body of `start_server_inner`.
- **Server subcommand** (`src/cli/spec.rs`): upstream turned `server_command`
  into a `let command` chain for its Windows-only `--allow-unelevated-clients`
  flag. The fork's `include_live_handoff` parameter is kept, and the chain is
  rebound so the flag applies to the full command. **Collision:**
  `server_command`.

### 2026-10-03 sync: conflict resolutions

Sixteen files conflicted; `README.md` is fork-owned and kept. The rest:

- **Worktree groups** (`src/client/shell/{sidebar,context_menu,overlay_input,mouse}.rs`):
  upstream fixed the same "repository parent counted as a worktree child"
  bug as fork #64 with its own grouping code, plus a `close_group` flag on
  workspace close. The fork's `group_parent_index` / `group_child_indices`
  grouping is kept. Upstream's `workspace_close_is_group` now answers
  through the fork's `parent_group_key_ref`, so its context-menu label
  ("Close group" vs "Close") and close requests use the fork's scope. An
  explicit single close (`close_group: Some(false)`) closes one space. The
  fork's merge confirmation and upstream's `request_workspace_close` both
  stay. Dragging a group onto one of its own members is now a no-op
  (upstream's guard on the fork's member list). **Collision:**
  `workspace_entries`, `displayed_workspace_status`, `open_close_confirmation`.
  Server side, `workspace_group_close_indices` keeps the fork's scope (first
  non-linked space plus its linked worktrees), so upstream's
  `duplicate_repo_parents_close_independently_unless_group_is_explicit` is
  rewritten to the fork's outcomes: the duplicate always closes alone, a
  group close on the parent leaves the duplicate open, and an implicit close
  of the parent is still refused. Upstream's two client tests from the same
  change (`duplicate_repo_parents_remain_visible_and_focusable_when_collapsed`,
  `duplicate_repo_parent_drag_does_not_target_its_own_move_block`) are
  adapted the same way: the duplicate has no group toggle, Close is found by
  action because the fork's menus carry move items, a menu whose target
  changes group shape closes instead of acting, and dragging the parent
  before the duplicate moves the parent alone. The duplicate-root section
  upstream added to `workspace_drag_moves_parent_worktree_as_one_block_and_rejects_child`
  now expects the duplicate to move alone (`["ws_duplicate"]`), not as part of
  the parent's block.
- **Codex hooks** (`src/integration/{mod,targets,tests}.rs`,
  `assets/codex/herdr-agent-state.{sh,ps1}`): upstream added an `Interrupt`
  hook that reports `idle` and requires a transcript only on `SessionStart`.
  The fork's richer hook (working/blocked/idle/metadata and titles) is kept
  with both of those added. `CODEX_INTEGRATION_VERSION` goes to 10 so
  installs from either side's v9 reinstall; `codex_v2_integration_status_is_outdated`
  now reads the constant instead of a literal 9. **Collision:** the codex hook
  scripts and `CODEX_STATE_HOOK_EVENTS`.
- **Server probe** (`src/server/autodetect.rs`): upstream made
  `is_server_listening_at` return `io::Result<bool>`. It stays `pub(crate)`
  for the fork's `server_not_running` split (`src/cli.rs`,
  `src/cli/status.rs`), which treats an error as "not listening" and logs it,
  as before.
- **Env locks** (`src/session.rs`, `src/update.rs`): upstream's new tests
  kept; `update.rs`'s private `env_lock` routed back through
  `test_config_env_lock`.
- **Flaky upstream test** (`src/app/custom_commands.rs`):
  `shell_command_invocation_executes_endpoint_owned_definition` polled for
  the file to exist, but the shell's `>` creates it empty before `printf`
  writes, so CI read `""`. It now waits for the content.
- **Docs** (`docs/next/website/.../{agents,integrations}.mdx`): upstream's
  rewritten agent table and status text, plus the fork's Codex hybrid
  paragraph. `skills/herdr/SKILL.md` keeps the fork's split skill; upstream's
  0.9.3 skill text (machine forwarding, agent start guidance) is **not yet**
  ported into the fork's `references/`.

### 2026-09-29 sync: conflict resolutions

Six files conflicted; two were the fork-owned `README.md` and
`.github/workflows/release.yml`, kept as the fork's. The rest:

- **libghostty-vt build** (`build.rs` → `crates/ghostty-vt/build.rs`):
  upstream moved the zig build into a `ghostty-vt` workspace crate that links
  from `OUT_DIR`. The fork's macOS `libtool` realign now writes the aligned
  archive straight into that crate's static-only link directory; the root
  `build.rs` is upstream's. `scripts/docker-check.sh` now touches the crate's
  build script to force the warm-up zig build.
- **API accept loop** (`src/api/server.rs`): upstream fixed the same bug as
  fork #71 with a fixed 50 ms `run_accept_loop`. The fork's `AcceptBackoff` /
  `SpawnDrops` loop is kept and upstream's function, constant and its two
  tests are dropped. **Collision:** any upstream edit to the accept loop.
- **Named terminal lookup** (`src/app/terminal_targets.rs`): upstream's keyed
  `terminals.get(id)` combined with the fork's live-name gate, and applied to
  the fork's stale-name path too.
- **Multiple prefix keys** (`src/input/keybind_help.rs`,
  `src/client/shell/palette.rs`): `keybind_help_groups` takes a prefix slice
  now; the palette passes `&keybinds.prefix` and upstream's
  `help_lists_every_configured_prefix` test reads the fork's entry struct.
- **Headless tests** (`src/server/headless/tests/mod.rs`): both modules kept.

### 2026-09-25 sync: conflict resolutions (`38b3ac7`)

Sixteen files conflicted. The decisions that change behavior, and where the
next sync is likely to conflict again:

- **Navigator pickers** (`src/client/shell/{overlays,overlay_input,navigator_moves,state}.rs`):
  upstream replaced the Go-to tree with a flat list and removed the `Tab`
  target and workspace expand/collapse. The fork's pane/tab move and merge
  pickers now sit on that list; tab destinations resolve through pane rows.
  **Collision:** upstream edits to `aggregate_navigation::navigator_rows`,
  the navigator footer, or `move_navigator_workspace` conflict here.
- **Sidebar row colors** (`src/client/shell/{sidebar,endpoint_sidebar}.rs`):
  both sides added `workspace_selection_background`. Upstream's keeps the
  name; the fork's drop-target variant is `workspace_highlight_background`.
  **Collision:** `render_workspace_rows`' signature.
- **Worktree lookup** (`src/app/api/worktrees.rs`): the fork's
  focused-space preference (`preferred_workspace_idx`) kept, with upstream's
  "explicit worktree provenance wins" applied inside its filter. Upstream
  removed `list_source_workspace_idx_for_space`; the fork dropped it too.
- **Tab close** (`src/client/shell/actions.rs`): upstream's last-tab
  confirmation owns `CloseTab`; the fork's action-table arm was removed.
- **Compile follow-ups:** `ClearPane` palette id `core:clear-pane`;
  `stream_active` removed with upstream's graphics API; the `close_tab`
  test finds the Close row by label because the fork's tab menu is longer.
- **Build:** this session's `cargo build` needed the Zig dependencies
  prefetched (`just zig-prefetch`); see that entry below.

## Peek a pane from another tab as a temporary overlay (issue [#95](https://github.com/cameronsjo/herdr/issues/95), Linear CAM-438)

### feat(pane): show a pane from another tab as a temporary overlay

- **PR:** [cameronsjo/herdr#147](https://github.com/cameronsjo/herdr/pull/147)
- **Files:** `src/app/{popup,state,runtime,api}.rs`, `src/app/api/panes.rs`, `src/api/schema{,/operations}.rs`, `src/cli/pane{,/bindings}.rs`, `src/server/{client_shell,headless/client_views,headless/render}.rs`, `src/ui/panes.rs`, `src/server/headless/tests/fork_peek.rs`, `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/{cli-reference,socket-api}.mdx`, `openspec/specs/pane-peek/spec.md` (via the archived `pane-peek` change)
- **Adds:** `pane.peek` / `herdr pane peek <pane_id>` shows a pane from another tab or space in the popup overlay over the current tab, with input and mouse routed to it; `pane.peek_close` / `herdr pane peek --close` closes it. `PopupPaneState.peek` marks a popup that borrows a tiled pane's terminal instead of owning one, so closing it shuts nothing down. `AppState::tab_resize_locked` keeps tab layout from resizing a peeked terminal, and a public request that opens or closes a peek reapplies tab geometry so the pane gets its layout size back. A peek closes when its terminal shuts down, and the popup branch of `PaneDied` no longer skips the pane's own exit handling for a peek. Server only: the client already renders and routes popups, so no wire codec changes.
- **Collision:** `PopupPaneState` and `close_popup_pane` in `src/app/{state,popup}.rs`, the popup branch of `PaneDied` in `src/app/api.rs`, `shutdown_terminal_runtime` in `src/app/runtime.rs`, `any_shell_surface_contains_pane` in `render.rs`, `public_request_may_change_geometry` and the end of `handle_api_request_with_shutdown_check` in `client_views.rs`, the popup title in `render_popup_surface`, and the four `tab_resize_locked` checks in `src/ui/panes.rs`. The popup invariant in `assert_invariants_for_test` now allows an attached terminal only for a peek.
- **Regression check:** `cargo nextest run -E 'test(/peek/)'`. Live: with an agent in tab 2, run `herdr pane peek <its pane id>` from tab 1; the agent shows in an overlay titled `peek: …`, keys reach it, and `herdr pane peek --close` returns it to tab 2 at its layout size.
- **Render scaling:** none added to pane loops; the lock check is one hash lookup and one comparison per pane resize.

## Drag a pane onto another pane to rearrange it (Linear CAM-396)

### feat: drag a pane by its frame to swap it with another pane

- **PR:** [cameronsjo/herdr#138](https://github.com/cameronsjo/herdr/pull/138)
- **Files:** `src/client/shell/{mouse,state,composition}.rs`, `src/client/shell/tests/mouse_selection.rs`, `docs/next/website/src/content/docs/concepts.mdx`, `openspec/specs/layout-moves/spec.md` (via the archived `drag-pane-to-rearrange` change)
- **Adds:** a left press on a pane's top, bottom or left frame (off any split handle; the right side beside the terminal rows is left to the scrollbar lane, which is unpublished while idle) becomes the same pane press an agent row makes, so it focuses on release or grows into a pane drag. `ClientPaneDropTarget::Pane` makes every other pane a drop target, outlined in the tab bar's drop-target colors; a same-tab drop sends `pane.swap` (explicit source and target) then `pane.focus`, a cross-tab drop opens the split chooser against that pane. Client only, built on existing methods, so it is a candidate to offer upstream.
- **Collision:** `pane_drop_target_at` (now takes the source pane id) and the left-press pane branch in `mouse.rs`, `ClientPaneDropTarget` in `state.rs`, the drop-target overlay pass before `render_link_hover` in `composition.rs`, and the agent-row sentence in `concepts.mdx`.
- **Regression check:** `cargo nextest run -E 'test(/pane_frame/) | test(/agent_row_click/) | test(/pane_dragged_onto/)'`. Live: in a two-pane tab, drag one pane's top border onto the other; the target is outlined, and on release the panes trade places.
- **Render scaling:** the overlay pass runs only while a pane drag has a pane target, once per frame, over one pane's outline.

## Codex state comes from screen detection only, as upstream (issue [#114](https://github.com/cameronsjo/herdr/issues/114))

### fix: drop the Codex state hooks and keep the title hook

- **PR:** [cameronsjo/herdr#136](https://github.com/cameronsjo/herdr/pull/136)
- **Files:** `src/agent_resume.rs`, `src/app/actions.rs`, `src/terminal/state.rs`, `tests/cli/hooks.rs` (upstream's #5278 state side re-applied), `src/integration/{targets,tests}.rs`, `src/integration/assets/codex/herdr-agent-state.{sh,ps1}`, `docs/next/website/src/content/docs/{agents,integrations}.mdx`, `openspec/specs/codex-lifecycle/spec.md` (via the archived `codex-screen-only-state` change)
- **Replaces:** the 2026-10-15 sync's full revert of upstream #5278. `herdr:codex` is a reserved native-state source again and the fork's visible-working override in `TerminalState` is gone, so those three files match upstream for Codex. Install no longer registers the `working`, `blocked` and `idle` hooks and removes them from an older install; the hook scripts accept only `session` and `metadata`. The fork's pane title (cameronsjo/herdr#26) is kept. Reason: both sides ship the same `codex.toml`, which reads Codex's terminal title (`Action Required`, the spinner) first, the screen already won every disagreement in the hybrid, and a shared Codex daemon misroutes hooks. `CODEX_INTEGRATION_VERSION` stays 11, unreleased (v0.9.3-palette.1 ships 10).
- **Collision:** only the title hook now: the Codex hook scripts, `CODEX_METADATA_HOOK_EVENTS` and the Codex install and uninstall in `targets.rs`.
- **Regression check:** `cargo nextest run -E 'test(/codex/)'`; `install_codex_removes_state_hooks_from_previous_integration` and `codex_hook_sends_no_state_reports` cover the drop. Live: `codex-state-probe.sh <pane>` (on #114) shows `agent get` and `agent explain` agree through prompt, approval, finish and Esc.
- **Render scaling:** not affected; removes a branch from state recompute.

## `sync-upstream.sh` keeps every fork-owned file, conflict or not

### fix: restore fork-owned files after every manual upstream sync

- **PR:** see the PR that adds this entry
- **Files:** `scripts/sync-upstream.sh` (`FORK_OWNED` and the merge), `scripts/test_sync_upstream.py`, `openspec/specs/fork-maintenance/spec.md` (via the archived `sync-upstream-keeps-fork-owned-files` change)
- **Replaces:** nothing upstream; the script is fork-only. `sync-upstream-ci.sh` already restores `README.md` and `.github/workflows/release.yml` from the fork's tip after every merge, because a conflict-only rule leaks. The manual script kept only `README.md`, and only on a conflict, so upstream's release workflow, or a README edit that merged cleanly, came through a manual sync. It now merges with `--no-commit --no-ff`, restores the same list (a test keeps the two lists equal), and then commits.
- **Regression check:** `python3 -m unittest scripts.test_sync_upstream` (5 tests). `test_fork_owned_files_keep_the_forks_copy` (a conflicted README and a cleanly merged release workflow both keep the fork's copy) and the list-parity test were confirmed red against the old script. `shellcheck --severity=warning scripts/sync-upstream.sh` is clean.
- **Render scaling:** not affected; maintenance script only.

## The Codex PowerShell hook keeps a quoted title in one argument

### fix: escape a Codex title for Windows PowerShell's argument passing

- **PR:** see the PR that adds this entry
- **Files:** `src/integration/assets/codex/herdr-agent-state.ps1` (`ConvertTo-NativeArgument`, the `report-metadata` call)
- **Replaces:** nothing upstream; the Codex title is the fork's (cameronsjo/herdr#26). The installed hook runs under Windows PowerShell 5.1, whose legacy native-argument passing does not escape a double quote inside an argument. A prompt like `fix "foo bar" now` reached herdr as `--title "fix foo" "bar now"`, a wrong title plus a stray argument, and `say "hi"` lost its quotes. The title is now escaped by the Windows argv rules when legacy passing applies (PowerShell 5.1, or `$PSNativeCommandArgumentPassing = "Legacy"`); under 5.1 a trailing backslash on a title with whitespace is doubled too. Standard passing in PowerShell 7.3+ already escapes, so the title goes through unchanged there. The `.sh` hook sends JSON and is unaffected. `CODEX_INTEGRATION_VERSION` stays 11: it is already one past the last fork release (v0.9.3-palette.1 ships 10).
- **Regression check:** no Windows runner covers the hook, so this was checked by hand with pwsh 7.4 on Linux, running the real hook with a fake `herdr` that logs its argv and `$PSNativeCommandArgumentPassing` set to `Legacy` and then `Standard`. Before, under Legacy: `fix "foo bar" now` gave `["fix foo", "bar now"]`. After, both modes give `fix "foo bar" now`, `say "hi"` and `x\"y z` unchanged, and a plain title is untouched. The 5.1-only trailing-backslash doubling follows PowerShell issue 1995 and was not run on 5.1.
- **Render scaling:** not affected; hook script only.
## The commit-msg hook lets a merge commit through

### fix: let the commit-msg hook pass merge commits

- **PR:** see the PR that adds this entry
- **Files:** `.githooks/commit-msg`, `scripts/test_commit_msg_hook.py` (new), `justfile` (`maintenance-test` runs it)
- **Replaces:** upstream's hook, which checks every commit subject with `scripts/conventional_commits.py --message-file`. Upstream squashes, so it never meets git's own `Merge ...` subject; the fork merges upstream syncs, so with `just install-hooks` in place every merge was refused, and `scripts/sync-upstream.sh` exited 1 even on a clean merge. The hook now exits 0 while `MERGE_HEAD` exists, the same exemption `--range` mode already makes with `--no-merges`. A non-merge commit is still checked.
- **Regression check:** `python3 -m unittest scripts.test_commit_msg_hook`: a clean `git merge --no-edit` and a merge committed after resolving a conflict both fail without the fix, and a non-conventional subject is still refused. On a sync that conflicts here, keep the `MERGE_HEAD` early exit.
- **Render scaling:** not affected; git hook only.

## The Codex PowerShell hook cuts a long title by character

### fix: count characters, not UTF-16 units, when cutting a Codex title

- **PR:** see the PR that adds this entry
- **Files:** `src/integration/assets/codex/herdr-agent-state.ps1` (`Normalize-Title`)
- **Replaces:** nothing upstream; the Codex title is the fork's (cameronsjo/herdr#26). The Windows hook measured and cut the title with `.Length` and `Substring`, which count UTF-16 units. A cut that landed inside an emoji's surrogate pair left half of it, which is not valid UTF-8, so herdr refused the argument and no title was reported. A title of 79 emoji (under the limit) was also cut. The `.sh` hook and the codex-lifecycle spec count characters ("over 80 characters to its first 77"); the `.ps1` now does too. `CODEX_INTEGRATION_VERSION` stays 11: it is already one past the last fork release (v0.9.3-palette.1 ships 10).
- **Regression check:** no Windows runner covers the hook, so this was checked by hand with pwsh 7.4 against `Normalize-Title` on its own. Before: 79 emoji came back as 38 emoji, a lone surrogate and `...`, and `"a"*76 + "😀" + "b"*10` ended in a lone surrogate. After: 79 emoji pass unchanged, the second ends `a😀...` at 80 characters, and ASCII titles of 80, 81 and 200 characters are unchanged from before.
- **Render scaling:** not affected; hook script only.
## The OpenSpec wrapper guards `store`, and the release cut reads every remote spelling

### fix: guard openspec store writes and accept ssh:// fork remotes

- **PR:** see the PR that adds this entry
- **Files:** `scripts/openspec.sh`, `scripts/test_openspec_wrapper.py` (new), `justfile` (`maintenance-test` runs it), `scripts/cut-fork-release.sh`, `AGENTS.md`
- **Replaces:** nothing upstream; both scripts are the fork's. The wrapper's guard refused the subcommands that reach outside the repo, but not `store`, whose `setup`, `register`, `unregister` and `remove` change the per-user store registry, and `store remove --yes` deletes a folder elsewhere on disk. They now need `OPENSPEC_ALLOW_OUTSIDE=1`, like `feedback`; `store list|ls|doctor` still run without it. So does `--store <id>` on any command, which works in a registered store's folder instead of this repo. `cut-fork-release.sh` read the remote's slug with a sed that knew only `git@host:` and `https://host/`, so `ssh://git@github.com/cameronsjo/herdr.git` and a trailing-slash HTTPS URL were refused as some other repo, while any host (or a bare `cameronsjo/herdr` path) passed. It now accepts the scp-style, `ssh://` and HTTPS spellings of github.com only.
- **Regression check:** `python3 -m unittest scripts.test_openspec_wrapper` (fails without the fix: the `store` writes reach the CLI). The slug check was run by hand: the github.com scp-style, `ssh://` (with and without `:22`) and HTTPS spellings, with and without `.git` or a trailing slash, give `cameronsjo/herdr`; `cameronsjo/herdr-mirror`, another host and a bare path are refused.
- **Render scaling:** not affected; maintenance scripts only.

## The Codex metadata hook skips an ephemeral session again

### fix: skip codex metadata on a SessionStart with no transcript

- **PR:** see the PR that adds this entry
- **Files:** `src/integration/assets/codex/herdr-agent-state.{sh,ps1}` (the transcript guard), `tests/cli/hooks.rs`
- **Replaces:** nothing upstream; the Codex `metadata` action is the fork's (cameronsjo/herdr#26). The scripts skipped a `SessionStart` with no `transcript_path` only for the `session` action, but `metadata` also runs on `SessionStart`, so an ephemeral Codex session still started `codex app-server` and reported a pane title. The codex-lifecycle spec already says such a payload MUST exit without reporting. The guard now tests the event, not the action. `CODEX_INTEGRATION_VERSION` stays 11: it is already one past the last fork release (v0.9.3-palette.1 ships 10).
- **Regression check:** `cargo nextest run -E 'test(/codex_metadata_hook/)'`: `codex_metadata_hook_prefers_app_server_thread_name` now also asserts that a transcript-less `SessionStart` sends nothing. Checked by hand against a fake socket and a fake app-server: before the fix the `metadata` action sent `pane.report_metadata`, after it sends nothing, while a `SessionStart` with a transcript and a `Stop` without one still report.
- **Render scaling:** not affected; hook script only.

## `sync-upstream.sh` resumes a merge started on an earlier day

### fix: resume a sync whose conflicts were resolved after midnight

- **PR:** see the PR that adds this entry
- **Files:** `scripts/sync-upstream.sh` (the worktree pick), new `scripts/test_sync_upstream.py`, `justfile` (`maintenance-test` runs it), `openspec/specs/fork-maintenance/spec.md` (via the archived `sync-upstream-resume-across-days` change)
- **Replaces:** nothing upstream; the script is fork-only. The branch and worktree are named `sync-upstream-<today>`, so a sync stopped on conflicts one day and rerun after midnight missed the hand-resolved worktree, created a new one from `fork/master`, hit the same conflicts and exited 1. The script now resumes any `sync-upstream-*` worktree with a merge in progress first, and refuses when there are two.
- **Regression check:** `python3 -m unittest scripts.test_sync_upstream` (3 tests: a later-day resume finishes the first merge and creates no new worktree, a same-day resume still finishes, two in-progress merges refuse). The later-day and two-merge tests were confirmed red against the old script. `shellcheck --severity=warning scripts/sync-upstream.sh` is clean.
- **Render scaling:** not affected; maintenance script only.

## Two fork scripts fail with their own guidance again

### fix: read the real plugin registry under XDG_CONFIG_HOME, and keep cut-fork-release's digest message

- **PR:** see the PR that adds this entry
- **Files:** `scripts/palette-live-check.sh` (`REAL_REGISTRY`), `scripts/cut-fork-release.sh` (step 9's digest read)
- **Replaces:** nothing upstream; both scripts are fork-only. `palette-live-check.sh` looked for the installed registry at `~/.config/herdr/plugins.json`, but herdr's `config_dir()` uses `$XDG_CONFIG_HOME/herdr` whenever that variable is set, so with it set the check refused with "No plugin registry" (or seeded from a stale `~/.config` copy). It now follows the same rule. In `cut-fork-release.sh`, a release body with no sha256 made `grep` exit 1, and under `set -euo pipefail` the script ended at "Reading the published digest" with no message; the `die` that says to verify the tap by hand now runs.
- **Regression check:** `shellcheck --severity=warning` on both scripts. The digest read was checked in isolation under `set -euo pipefail` with a body holding no digest: the old line exits silently, the new one reaches the `die`. The registry path expansion was checked with `XDG_CONFIG_HOME` set (it resolves under that directory) and unset (it resolves under `~/.config`).
- **Render scaling:** not affected; maintenance scripts only.

## docker-check counts a test killed by a signal as a failure

### fix: count every nextest failing status in docker-check

- **PR:** see the PR that adds this entry
- **Files:** `scripts/docker-check.sh` (`classify_nextest_failures` and the main body), `scripts/test_docker_check.py`, `openspec/specs/fork-maintenance/spec.md` (via the archived `docker-check-every-failure-status` change)
- **Replaces:** nothing upstream; `docker-check.sh` is fork-only. The parser read only `FAIL` lines. nextest reports a signal-killed test as `SIGABRT`/`SIGSEGV` (a stack overflow or a double panic aborts), a timeout kill as `TIMEOUT`, a Windows abort as `ABORT` and a leaked handle as `LEAK-FAIL`, so such a run parsed as zero failures and ended in `docker-check: PASS`. Those statuses now go through the same allowlist and retry path as `FAIL`. A nextest run that exits non-zero with nothing recognized now fails the check outright, so a status this parser does not know yet cannot pass either.
- **Regression check:** `python3 -m unittest scripts.test_docker_check` (11 tests). `test_signal_timeout_and_leak_statuses_count_as_failures` was confirmed red against the old parser. `shellcheck scripts/docker-check.sh` reports only the pre-existing SC1091 info.
- **Render scaling:** not affected; maintenance script only.

## `upstream-drift.sh` answers when upstream has no `v*` tag

### fix: keep upstream-drift running when no upstream tag matches

- **PR:** see the PR that adds this entry
- **Files:** `scripts/upstream-drift.sh` (the tag pick only), new `scripts/test_upstream_drift.py`, `justfile` (`maintenance-test` runs it)
- **Replaces:** nothing upstream; the script is fork-only. The newest-tag pick ended in `grep -v -- '-palette\.' | head -1`. With no `v*` tag, or only `-palette.` tags, grep matched nothing and exited 1, and under `set -euo pipefail` the assignment ended the script: no output and exit 1, so a level fork read as "behind" and the `(none found ...)` fallback never printed. The pick is now one `awk` that cannot fail on an empty match.
- **Regression check:** `python3 -m unittest scripts.test_upstream_drift` (4 tests: level with no tags exits 0, palette-only tags read as none found, behind names the newest non-palette tag and exits 1, a missing `fork` remote exits 2). The first two were confirmed red against the old script.
- **Render scaling:** not affected; maintenance script only.

## `rows_by_agent` set while grouped warns at config load (issue [#31](https://github.com/cameronsjo/herdr/issues/31))

### feat(config): warn when rows_by_agent is ignored while grouped

- **PR:** see the PR that adds this entry
- **Files:** `src/config/sidebar.rs` (new `AgentsSidebarConfig::grouped_rows_by_agent_diagnostic`, two tests), `src/config.rs` (one `.chain(...)` line in `Config::collect_diagnostics`), `docs/next/website/src/content/docs/configuration.mdx` (one sentence, English only), `openspec/specs/agent-sidebar/spec.md` (via the archived `warn-rows-by-agent-while-grouped` change)
- **Replaces:** nothing upstream. Since #23, `grouped_rows` wins over every `rows_by_agent` override while `group_by` groups entries, so an override did nothing with no signal. Config load now reports `ui.sidebar.agents.rows_by_agent is ignored while group_by groups entries (overrides for: <ids>); grouped entries use ui.sidebar.agents.grouped_rows`. The wording avoids `config_diagnostic_summary`'s "using defaults" and "keeping current config" classes, so the banner reads as a plain warning and the config loads as written.
- **Regression check:** `cargo nextest run -E 'test(/grouped_rows_by_agent_diagnostic/)'`. Dropping the `.chain` line in `collect_diagnostics` reddens `grouped_rows_by_agent_diagnostic_names_ignored_overrides`. **Collision:** an upstream edit to `Config::collect_diagnostics`; keep the fork's `.chain(self.ui.sidebar.agents.grouped_rows_by_agent_diagnostic())` line.
- **Render scaling:** not affected; runs once per config load.

## `agent start --env` sets variables for the agent process only (issue [#103](https://github.com/cameronsjo/herdr/issues/103))

### feat(agent): set environment variables for the agent process in agent start

- **PR:** [cameronsjo/herdr#124](https://github.com/cameronsjo/herdr/pull/124) (with its review follow-up)
- **Files:** new fork file `src/api/schema/fork_agent_env.rs` (the `AgentEnv` map with a key-only `Debug` and a hand-written `Deserialize` that never quotes a mistyped value back, the key grammar, value checks and pair parsing); `src/api/schema.rs` (module line); `src/api/schema/agents.rs` (`AgentStartParams.env`); `src/api/schema/server.rs` (`ServerCapabilities.agent_start_env`) and its literals in `src/api/server.rs` (`true` in `default_capabilities`), `src/cli/status.rs`, `src/update.rs`, `src/api/schema/tests.rs`; `src/app/agents.rs` (`start_agent` validates `env`, refuses unsupported shells, and calls `interactive_agent_shell_command`; `AgentStartError::{InvalidEnv, EnvUnsupportedShell}`); `src/platform/mod.rs` (new `interactive_agent_shell_command`, `interactive_unix_shell_command_with_env`, `unix_agent_env_shell_supported`, `pane_shell_label`, `powershell_env_script`, fish quoting), `src/platform/{linux,macos,windows,fallback}.rs` (new `interactive_shell_command_with_env` and `agent_env_shell_supported` beside the unchanged `interactive_shell_command`); `src/cli/agent.rs` and `src/cli/agent/bindings.rs` (`--env`, the capability gate); `src/cli/spec.rs` (upstream's `spec_models_agent_start_target_and_trailing_args` listed `--env` as a removed legacy option and now asserts it is present); `docs/next/api/herdr-api.schema.json`; `docs/next/website/src/content/docs/cli-reference.mdx` (English only; ja and zh-cn not yet translated); `openspec/specs/agent-launch/`.
- **Replaces:** nothing upstream. `agent start` types its command into the pane's shell, so the variables are written into that command line, only for shells whose quoting is verified against the real shell: `KEY='value' agent ...` with POSIX quoting for sh, bash, dash, zsh, ksh and mksh; the same prefix with fish's own quoting (`\\` and `\'` escaped inside single quotes) for fish; and a save-and-restore `try`/`finally` wrapper, with an ordinal (case-sensitive) dictionary, for pwsh and for every Windows pane (inside the encoded script on cmd). nu, xonsh, elvish, csh and tcsh are refused with `agent_env_unsupported_shell` (naming the shell, never a value) before anything is typed: POSIX quoting there executed part of a value in fish, nu, elvish and xonsh, and csh-family shells expand `!` inside single quotes. With no `env` the typed command is byte-identical to upstream's in every shell. Keys must match `^[A-Za-z_][A-Za-z0-9_]*$`; a CLI pair without `=` is a usage error; values may not hold control characters (Cc, including U+0085), U+2028, U+2029 or format characters such as bidi overrides (`invalid_agent_env`). No error or `Debug` output contains a value, a malformed key is not echoed, and an `env` of the wrong JSON type gets `invalid_request` without the value. **Compatibility:** `env` is optional and omitted when empty, so old clients and requests are unchanged. An older server would drop `env` and still succeed, so the server advertises `capabilities.agent_start_env` and the CLI refuses `--env` with `agent_start_env_unsupported` when it is missing. Raw API callers should check the same flag. **Worth knowing:** the value is visible in the pane, may land in shell history, and sits in the `herdr` CLI process's argv while the call runs. Env is not persisted, so an agent resumed by a session restore starts without it (follow-up to be filed). Under PowerShell an empty value removes the variable instead of setting it empty.
- **Regression check:** `cargo nextest run -E 'test(/agent_start_env/) + test(/agent_env_round_trips/) + test(/fork_agent_env/) + test(/interactive_agent_shell_command/) + test(fish_quoting_escapes_backslash_and_quote) + test(powershell_env_script_doubles_every_single_quote_form) + test(agent_start_unsupported_shell_error_names_the_shell_only) + test(spec_models_agent_start_target_and_trailing_args)'`. The `agent_env_round_trips_in_real_*` tests run 27 adversarial values, plus the empty value outside pwsh (quotes, backslashes, a trailing backslash, `$()`, backticks, `!`, `%`, `^`, `;`, `#`, typographic quotes, the injection payloads from the PR #124 review) through each of sh, bash, dash, zsh, ksh, mksh, fish and pwsh that is installed (a missing shell is skipped), and require the agent to receive each value byte for byte, the shell not to keep it, and no `PWN` file to appear. Staged break, 2026-10-23: quoting fish values with the POSIX quoter reddens `agent_env_round_trips_in_real_fish`. On a sync, `interactive_agent_shell_command_without_env_is_unchanged` must stay green (no-env launches still go through upstream's `interactive_shell_command`), and any new `ServerCapabilities` literal from upstream needs `agent_start_env`. **Collision:** upstream edits to `AgentStartParams`, `start_agent`, `ServerCapabilities`, the `agent start` CLI binding, the `agent start` section of `cli-reference.mdx`, or the legacy-option list in `src/cli/spec.rs`.
- **Render scaling:** not affected; the change runs once per `agent.start` call.

## `parse_workspace_id` takes upstream's text again

### fix: use upstream's bound in parse_workspace_id

- **PR:** see the PR that adds this entry
- **Files:** `src/app/ids.rs` (`parse_workspace_id` only)
- **Replaces:** the fork's own copy of the positional-id bound (#77, PR #81). Upstream added the same `.filter` in #5171; the fork's `|index| *index <` form and its four-line comment are now upstream's `|&index| index <`, with the same behavior. The function matches upstream `master` (`143bf5f`) exactly. The #81 entry below is marked retired for that part.
- **Regression check:** `cargo nextest run --locked -E 'test(positional_workspace_id) + test(numeric_workspace_ids_beyond_the_last_workspace_are_not_found)'`. On a sync, `parse_workspace_id` should merge with no conflict.
- **Render scaling:** not affected; the parser runs once per request.

## The OpenSpec wrapper localizes the skills it regenerates

### fix: run openspec-localize after openspec init and update

- **PR:** see the PR that adds this entry
- **Files:** `scripts/openspec.sh`, `scripts/openspec-localize.sh` (header comment), `AGENTS.md` (fork section only)
- **Replaces:** nothing upstream. `openspec init` and `openspec update` rewrite the generated skills with a bare, unpinned `Bash(openspec:*)` grant, and the `OpenSpec` workflow fails until `scripts/openspec-localize.sh` narrows them again. That second command was a manual step in AGENTS.md. The wrapper now runs the CLI without `exec` for those two subcommands, then runs the localize script, even after a failed CLI run. The CLI's failure wins the exit status; otherwise the localize script's status is returned. Every other subcommand still `exec`s the CLI.
- **Regression check:** `scripts/openspec.sh update`, then `bash scripts/openspec-localize.sh && git diff --exit-code -- .claude .agents/skills` (the workflow's own check) exits 0. On 2026-10-20 the old wrapper's `update` left 18 generated files changed and this one left none. `scripts/openspec.sh init --no-such-flag` exits 1.
- **Render scaling:** not affected; no code changes.

## A test checks that every env lock is the shared one

### test: fail when a module's env_lock is not the shared lock

- **PR:** see the PR that adds this entry
- **Files:** new fork file `scripts/test_env_lock_architecture.py`; `justfile` (`maintenance-test` lists it)
- **Replaces:** nothing upstream. Routing each module's test `env_lock()` through `crate::config::test_config_env_lock()` (#74, #36) was a manual step at every sync, and the 2026-10-03 sync had to re-route `update.rs` by hand. The test scans `src/` for every `fn *env_lock(` other than the shared lock itself. Each one's body must call `crate::config::test_config_env_lock()`, must not build its own `static`, `Mutex::new`, `OnceLock` or `LazyLock`, and must not name any `static` lock declared anywhere in `src/` (so `&LOCK` or `&super::LOCK` beside the shared call still fails). A `'static` lifetime is not a `static` declaration. It reuses `mask_comments_and_literals` and `function_body` from `scripts/test_ui_hot_path_architecture.py`, so a comment or string naming the lock does not count.
- **Regression check:** `python3 -m unittest scripts.test_env_lock_architecture`. Staged break, 2026-10-20: giving `src/update.rs`'s `env_lock()` its own `static LOCK` reddens it with `src/update.rs:2610: env_lock()`, and so does a module-level `static PRIVATE_ENV_LOCK` that `env_lock()` returns after calling the shared lock. A wrapper binding `let lock: &'static Mutex<()> = crate::config::test_config_env_lock();` stays green. On a sync that conflicts here, route any new `env_lock()` through the shared lock until the test passes. **Collision:** an upstream rename of the two helpers it imports breaks the import loudly.
- **Render scaling:** not affected; a test only.

## A PR title edit reruns the title check

### ci: check the PR title again when it is edited

- **PR:** see the PR that adds this entry
- **Files:** new fork file `.github/workflows/pr-title.yml`, `AGENTS.md` (fork section only)
- **Replaces:** nothing upstream. ci.yml's `conventional-commits` job checks the PR title on `opened`, `synchronize` and `reopened` only, so a corrected title got no new result until the next push. The fork-only `PR title` workflow runs the same `python3 scripts/conventional_commits.py "$PR_TITLE"` check on those events and on `edited`. ci.yml is unchanged, so its own check still keeps its old result until a push.
- **Regression check:** `actionlint .github/workflows/pr-title.yml` exits 0. On a PR, editing the title to `bogus: x` turns `PR title` red, and editing it back turns it green, with no push. On a sync, confirm `pr-title.yml` survived the merge.
- **Render scaling:** not affected; no code changes.

## The fork ledger is tracked without `git add -f`

### chore: stop ignoring docs/fork

- **PR:** see the PR that adds this entry
- **Files:** `.gitignore` (two `!/docs/fork/` exceptions beside upstream's `docs/next`, `docs/preview` and `docs/versions` ones), `AGENTS.md` (fork section only: the `git add -f` note is gone)
- **Replaces:** nothing upstream. Upstream's `/docs/*` rule ignored the fork's own ledger, so every fork PR needed `git add -f`, and a plain `git add` in an `&&` chain failed the rest of the chain.
- **Regression check:** `git check-ignore --no-index docs/fork/CHANGES.md` exits 1. Without `--no-index`, git skips tracked paths and exits 1 even while `/docs/*` still ignores the file. **Collision:** an upstream edit to the `/docs/*` block of `.gitignore`; keep both `!/docs/fork/` lines.
- **Render scaling:** not affected; no code changes.

## `pane send-text` refuses a canonical-mode line the macOS tty would drop

`pane.send_text` skipped the `validate_input_submission` check that
`pane.send_input` runs. Darwin discards a canonical-mode line longer than
`MAX_CANON` (1024 bytes) with no error, so a long send-text to a pane in
canonical mode reported success and delivered nothing. The handler now runs the
same preflight and returns `pane_send_failed`. The mode is sampled at
admission, so a pane that flips to canonical while the text waits in the actor
queue can still drop a long line. Raw-mode panes are unaffected:
a 622 KB send-text through a throwaway server arrived byte-identical, even
with a reader that started three seconds late.

- **Files:** `src/app/api/panes.rs` (`handle_pane_send_text`).
- **Regression check:** `send_text_rejects_oversized_canonical_input_instead_of_dropping_it`
  (macOS only, `src/platform/macos/input_tests.rs`; it also pins that
  multi-line text under the limit and long raw-mode text are accepted).
  `actor_delivers_text_far_past_the_pty_buffer_to_a_slow_raw_reader`
  (`src/pty/actor/unix.rs`) is a characterization test of the unchanged
  raw-mode path past 1 MiB, not a pin for this check.
- **Collision:** any upstream change to `handle_pane_send_text` or
  `PtyInputGuard`.

## Fork specs live in OpenSpec (issue [#112](https://github.com/cameronsjo/herdr/issues/112))

### docs: adopt OpenSpec for fork-only capabilities

- **PR:** [cameronsjo/herdr#113](https://github.com/cameronsjo/herdr/pull/113)
- **Files:** `openspec/` (config, ten baseline specs, the archived `2026-10-20-adopt-openspec-baseline` change), `.claude/` and `.agents/skills/openspec-*` (generated, localized to the pinned CLI), `scripts/openspec.sh`, `scripts/openspec-localize.sh`, `scripts/openspec/` (CLI pin), `.github/workflows/openspec.yml` (fork-only), `AGENTS.md` (fork section only)
- **Replaces:** nothing upstream. Adds a spec per fork capability alongside this ledger; `docs/plans/` takes no new plans after 2026-10-20.
- **Regression check:** `scripts/openspec.sh validate --all --strict --no-interactive` passes. On a sync, drop any spec requirement for a behavior upstream now ships itself, and confirm `.github/workflows/openspec.yml` and the fork section of `AGENTS.md` survived the merge.
- **Render scaling:** not affected; no code changes.

## Moving a space's last tab closes the space (issue [#107](https://github.com/cameronsjo/herdr/issues/107))

### fix(api): move a space's only tab instead of refusing

- **PR:** [cameronsjo/herdr#108](https://github.com/cameronsjo/herdr/pull/108)
- **Files:** `src/app/api/tabs.rs` (`tab_move_to_workspace`, `tab_move_to_new_workspace`, new `take_tab_for_cross_workspace_move` and `can_close_workspace_for_last_tab_move`, `finish_cross_workspace_tab_move` takes an optional source), `src/api/schema/tabs.rs` (`TabMoveReason::LastTabInWorkspace` doc), `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/keyboard.mdx`
- **Replaces:** the fork's own `tab.move` refusal of a workspace's last tab (`changed: false, reason: last_tab_in_workspace`), which the client shell ignored, so `move tab to space` and `move tab to new space` silently did nothing on a one-tab space. The move now closes the emptied workspace the way `pane.move` closes one its last pane leaves: the workspace is removed before it is drained, `active`/`selected` shift past it, a view that was on it follows the tab without recording a focus change (the pane in focus is the same one), and a `WorkspaceClosed` event (no snapshot, as in `workspace.merge`) follows `TabMovedAcrossWorkspaces`. A workspace that owns linked worktree workspaces still refuses with `last_tab_in_workspace`, since closing it would orphan them. No wire shape changes.
- **Regression check:** `cargo nextest run -E 'test(/api_tab_move_/)'`. Restoring the unconditional refusal reddens `api_tab_move_last_tab_closes_its_workspace_and_view_follows`, `api_tab_move_last_tab_from_background_workspace_keeps_the_view`, `api_tab_move_last_tab_to_an_earlier_workspace_keeps_the_view`, `api_tab_move_last_tab_to_new_workspace_replaces_its_workspace` and `api_tab_move_only_tab_of_only_workspace_to_new_workspace`; dropping the worktree-owner gate reddens `api_tab_move_refuses_the_last_tab_of_a_worktree_group_owner`. Recording the follow as a focus change (dropping the `previous_pane_focus` restore in `follow_moved_tab`) reddens `api_tab_move_last_tab_closes_its_workspace_and_view_follows`.
- **Render scaling:** not affected; the change runs once per API call.

## Agents running outside a pane can register in the agents panel

### feat(agent): register pane-less agents with a heartbeat TTL

- **PR:** see the PR that adds this entry
- **Files:** new fork files `src/fork_registered_agents.rs` (registry, TTL, endpoint control), `src/api/schema/fork_registered_agents.rs` (params and info types), `src/app/api/fork_registered_agents.rs` (handlers), `src/cli/fork_registered_agents.rs` (CLI), `src/client/shell/fork_registered_agents.rs` (panel rows). One-line hooks in upstream files: `src/api/schema.rs` (three `Method` variants), `src/api/schema/response.rs` (three result variants), `src/api/server.rs` (method names), `src/app/api.rs` (dispatch), `src/app/{state,mod}.rs` (the `registered_agents` field), `src/app/actions.rs` (`next_agent_metadata_expiry` includes the registry), `src/app/runtime.rs` (`expire_metadata_at` expires it), `src/server/{clients.rs,headless/render.rs}` (per-client revision and send), `src/client/endpoint/control.rs`, `src/client/mod.rs`, `src/client/shell/{endpoints,sidebar,agent_sidebar}.rs`, `src/cli/{agent,spec}.rs`, `src/main.rs`, `src/cli.rs`, `src/client/shell.rs`, `docs/next/api/herdr-api.schema.json`
- **Replaces:** Nothing upstream. The agents panel lists only herdr panes, so a Claude session in cmux or a plain terminal could not appear there. New API methods `agent.register` (upsert keyed by `source` and `name`; repeating it is the heartbeat), `agent.unregister`, and `agent.registered`, with CLI `herdr agent register|unregister|registered`. A record expires when its `ttl_ms` (default 60000, 1000 to 86400000) passes without another register; the expiry rides the existing agent-metadata timer. At most 256 records, and the encoded list is budgeted at 512 KiB so the control stays well under the frame limit; `source` may not contain `/`, since row ids join `source` and `name` with one. `agent.list` is unchanged, since its `AgentInfo` requires pane ids. **Endpoint contract:** the registry reaches client shells as the optional endpoint control `fork.registered-agents.v1`, sent only when the registry revision changes (a heartbeat that changes nothing does not bump it), and never to a client before any agent has registered, so existing endpoint message sequences are unchanged. Older clients ignore unknown optional controls, and no snapshot codec, digest, or method-shape fixture changes. The client draws registered agents after the pane rows under one synthetic "elsewhere" space by feeding them through `agent_rows` as a small snapshot of their own; their `registered:` row ids get no click target. An active agent view leaves them after the view's rows (under their "elsewhere" header when agent grouping is in effect; unlabeled otherwise): the server evaluates a view against panes and sends the client only its label, so applying a view's filter to these rows would need server-side evaluation and a change to what the `v1` control carries. A sort-only view (an orchestrator pin) is therefore exact, while a filtering view also shows registered rows it would not match, and a view that matches no pane shows those rows instead of "no matching agents". The client decodes the control tolerantly (an unknown status reads as `unknown`, a malformed record is dropped, an unparseable payload clears the list), redraws when the list changes, and clears it when the endpoint disconnects. A control that fails to frame is skipped with a warning rather than dropping the client. The multi-machine endpoint agent list (`endpoint_agents.rs`) does not show them yet. Upstream edits to `render_agent_panel`'s signature or its row-hit closure, or to `expire_metadata_at`, will conflict here.
- **Regression check:** `cargo nextest run -E 'test(/fork_registered_agents/)'`. Staged breaks, each confirmed red then green on 2026-09-29: letting registered rows push to `hits.agents` reddens `panel_draws_registered_agents_without_click_targets`; dropping the `expire_registered_agents` call from `expire_metadata_at` reddens `expiry_runs_with_the_metadata_timer`. Making `registered_agent_rows` return nothing while `snapshot.agent_view_label` is set (the pre-2026-10-06 behavior, which also needs the snapshot parameter back) reddens `active_agent_view_keeps_registered_agents`, confirmed red then green on 2026-10-06.
- **Render scaling:** not measured. Per client per render the server adds one revision comparison; the client builds registered rows only when the endpoint reports any.

## Fork issue fixes riding the 2026-09-25 upstream sync

### Review-panel fixes on PR #87 (2026-09-25)

A six-seat cadence panel (code ×2, security, operability, developer and user
experience) reviewed the sync resolution and fork commits. Fixes:

- **Accept loop** (`src/api/server.rs`, #71): accept errors back off 10 ms→1 s
  and keep accepting instead of ending the loop; errors log kind and OS code;
  refused connection threads are counted. **Regression check:**
  `cargo nextest run -E 'test(accept_backoff_grows_to_a_cap_and_resets_on_success) + test(a_refused_connection_thread_drops_only_that_connection)'`.
- **Stale agent name** (`src/app/terminal_targets.rs`, #73): refusal keeps
  `agent_not_found` but names the pane and the next step; comments now name
  the handlers' own live-agent checks as the control.
- **Logs and ids** (#80, #44): restore and handoff escape untrusted strings;
  restore names new id, index, label and reason; `public_workspace_id` logs
  its caller; restored ids are capped at `u32::MAX`; the command-target check
  uses `try_public_workspace_id`. **Regression check:**
  `cargo nextest run -E 'test(canonical_workspace_ids) + test(distribution_change)'`.
- **Pickers** (`src/client/shell/`): mode titles, split chooser names its
  tab and pane, ←→ steps between spaces, merge footer and no source row.
  **Regression check:**
  `cargo nextest run -E 'test(left_and_right_step_between_spaces) + test(a_pane_row_destination_asks) + test(the_merge_picker_does_not_offer_the_source_space)'`.
- **Label workflow** logs the upstream refs it ignored.
- **Round 2** (fix-delta panel, 2026-09-25): the prefetch refuses any
  dependency whose hash or URL could inject git options or escape the work
  directory, and trusts a download (and its nested `build.zig.zon`) only
  once `zig fetch` computes the pinned hash; failures carry Zig's own error.
  The merge picker with no other space says so; spaces-only pickers search
  and count spaces; `←→` also stops on "new space"; the split chooser names
  the space and the row's label. The accept loop logs recovery, rate-limits
  dropped-connection warnings, and tests the every-50th log rule. The stale
  name refusal lists every holding pane and says "detects", not "running".
  **Regression check:**
  `python3 -m unittest scripts.test_prefetch_zig_deps` (includes the
  `git+--upload-pack` and `../` hash cases) and
  `cargo nextest run -E 'test(a_merge_picker_with_no_other_space_says_so)'`.
  Proven from an empty Zig cache 2026-09-25: 39 fetched, all hash-verified.
- **Round 3** (2026-09-25): prefetch failures keep curl's error when the
  git fallback also fails, name the failing git step and the route, label
  expected against computed hashes, and escape control characters; curl
  also pins redirects to https. Dropped connection threads are counted per
  episode, so a later episode is never silent. Recovery lines log only when
  a streak hid failures. A status filter that empties the merge picker reads
  "No matching spaces". A stale name held by several panes names each.
  **Regression check:**
  `cargo nextest run -E 'test(a_refused_connection_thread_drops_only_that_connection) + test(a_stale_name_on_several_panes_names_each_of_them) + test(a_filter_that_empties_the_merge_picker_is_not_called_no_other_space)'` and
  `python3 -m unittest scripts.test_prefetch_zig_deps`.
- **Round 4** (2026-09-25): the prefetch's error chain, git step naming,
  output escaping (now `format_failure`) and missing-binary handling are
  tested; each was confirmed red with its fix reverted. Accept and spawn
  recovery share `streak_hid_failures`, tested at streaks 0, 1 and 2, and a
  shutdown mid-streak logs the hidden count. Temp-repo tests in
  `test_preview.py` and `test_release.py` disable git auto-maintenance,
  which raced their cleanup in CI. **Regression check:**
  `python3 -m unittest scripts.test_prefetch_zig_deps scripts.test_preview scripts.test_release`
  and `cargo nextest run -E 'test(only_a_streak_that_hid_failures_gets_a_recovery_line)'`.
- **Zig prefetch** (`scripts/prefetch_zig_deps.py`, `just zig-prefetch`):
  fills the Zig cache when `build.rs` fails on a `build.zig.zon` fetch, as it
  did behind this session's proxy; also named in `sync-upstream.sh`'s next
  steps. **Regression check:** `python3 -m unittest scripts.test_prefetch_zig_deps`.

### test: share one env lock across every test that mutates the environment (#74, #36)

- **Files:** `src/config.rs` (`test_config_env_lock`), the test `env_lock()` in `src/{api/server,client/tests/mod,platform/linux,product_announcements,server/autodetect,session,update}.rs`
- **Replaces:** Upstream keeps a private `env_lock()` per module, which does not exclude other modules mutating the same variables. The fork routes all of them through one lock that clears poison on each call. `scripts/test_env_lock_architecture.py` fails when a sync brings in an `env_lock()` that does not; see its entry above.
- **Regression check:** `just test-single-process` (five single-process `cargo test --bin herdr` runs; each must reach `test result: ok`). `just test` cannot show this race: nextest runs every test in its own process. 3 of 8 runs failed before, 9 of 9 passed after, 2026-09-25. The follow-up sweep also moved `remote/attach.rs`'s `TMPDIR` lock and `integration_env_lock` onto the shared lock.

### test: point the hot-path architecture guard at the client shell (#40)

- **Files:** `scripts/test_ui_hot_path_architecture.py`
- **Regression check:** `python3 -m unittest scripts.test_ui_hot_path_architecture`. Staged break confirmed 2026-09-25: an `input_state` call appended to `src/client/shell/composition.rs` reddens it.

### test: guard palette plugin rows against stray bold cells (#62)

- **Files:** `src/client/shell/tests/palette.rs`
- **Replaces:** Nothing; a guard only. The report did not reproduce on the current tree.
- **Regression check:** `cargo nextest run -E 'test(an_unselected_plugin_row_name_carries_no_bold_cells)'`. Making unselected rows bold reddens it.

### fix: warn when a live handoff crosses between upstream and fork builds (#44)

- **Files:** `src/server/handoff.rs`
- **Replaces:** Upstream writes `source_protocol` and never reads it. The fork logs it on every accepted handoff and warns when `is_fork_protocol` differs between source and receiver. It does not refuse: protocols differ on every in-place upgrade.
- **Regression check:** `cargo nextest run -E 'test(handoff)'`. The warning itself has no test.

### fix: refuse to type into a pane whose agent name has no live agent (#73)

- **Files:** `src/app/terminal_targets.rs` (`resolve_agent_input_target`), `src/terminal/state.rs` (`agent_name_routes`), `src/app/api/agents.rs`
- **Replaces:** Upstream resolves `agent.prompt`, `send_keys` and `type_submit` names with no liveness gate. The fork requires a detected or reported agent, or a managed launch in flight. Read and wait paths are unchanged. Upstream edits to `resolve_agent_target` or those three handlers will conflict here.
- **Regression check:** `cargo nextest run -E 'test(agent_name_stops_routing_once_no_agent_backs_it) + test(agent_wait_tolerates_detection_uncertainty_and_pane_target_rename)'`. Staged break confirmed 2026-09-25: dropping the gate reddens the first.

### fix: stop public_workspace_id panicking and validate restored ids (#80)

- **Files:** `src/app/ids.rs`, `src/workspace.rs` (`is_canonical_workspace_id`, `reserve_workspace_id_strs`), `src/persist/restore.rs` (`restored_workspace_id`)
- **Replaces:** Upstream's `public_workspace_id` indexes bare and restore keeps any stored id. The fork adds `try_public_workspace_id`, makes the infallible form log and return an empty id, and replaces malformed or duplicate restored ids after reserving every canonical one.
- **Regression check:** `cargo nextest run -E 'test(restored_workspace_ids) + test(canonical_workspace_ids)'`. Staged break confirmed 2026-09-25: dropping the canonical check reddens it.

### fix: close the global menu when a snapshot changes its rows (#72)

- **Files:** `src/client/shell/global_menu.rs` (`reconcile_global_menu`), `src/client/shell/state.rs`, `src/client/shell/tests/startup_overlays.rs`
- **Replaces:** Upstream dispatches a global-menu click by index against the current snapshot while the rows were drawn from an earlier one. The fork closes the menu in the snapshot path when the item list would change, as `reconcile_context_menu` does.
- **Regression check:** `cargo nextest run -E 'test(global_menu_closes_when_an_update_row_arrives_under_it)'`. Staged break confirmed 2026-09-25: commenting out the `reconcile_global_menu` call reddens it.

### fix: keep the api accept loop alive when a connection thread fails (#71, partial)

- **Files:** `src/api/server.rs`
- **Replaces:** Upstream spawns each API connection with `std::thread::spawn`, which panics inside the accept loop on thread exhaustion, and logs the loop's exit at `debug`. The fork uses `thread::Builder` and drops only the failed connection, and warns when the loop exits while the server runs. The root mechanism in #71 is still open.
- **Regression check:** `cargo nextest run -E 'test(/api::/)'`. No test forces a thread-spawn failure.

### refactor: move valid_agent_name beside sanitize_label (#34)

- **Files:** `src/label.rs`, `src/app/{agents,mod}.rs`, `src/terminal/state.rs`, `src/persist/restore.rs`
- **Replaces:** Upstream has no `valid_agent_name`; it is the fork's (#18). It moved from `src/app/agents.rs` to `src/label.rs`. Upstream edits near either spot may conflict.
- **Regression check:** `cargo nextest run -E 'test(agent_names_use_a_small_cli_safe_grammar)'`.

### ci: skip upstream-synced commits when labelling fork issues (#60)

- **Files:** `.github/workflows/label-next-release-issues.yml`
- **Replaces:** Upstream scans every pushed commit. On the fork, the scan now excludes commits reachable from `herdrdev/herdr` `master` (fetched read-only).
- **Regression check:** `actionlint .github/workflows/label-next-release-issues.yml`; `git log --format=%b <before>..<after> --not upstream/master` over a sync range yields only fork refs (34 refs down to `#42` for the 2026-09-25 sync).

### docs: ja and zh-cn rows_by_agent wording (#42)

- **Files:** `docs/next/website/src/content/docs/{ja,zh-cn}/configuration.mdx`
- **Regression check:** manual read — neither locale claims `rows_by_agent` wins while grouped.

## Fork working notes for agent sessions

### docs(agents): record fork gotchas that cost sessions a CI cycle

- **PR:** see the PR that adds this entry
- **Files:** `AGENTS.md` (new `### Fork working notes` under the fork operating rule)
- **Replaces:** Nothing upstream. Fork-only guidance, kept inside the fork section so upstream syncs of `AGENTS.md` don't conflict with it.
- **Regression check:** manual read. After a sync, `AGENTS.md` still has the `### Fork working notes` list, and the commands it names (`just zig-prefetch`, `just test-single-process`, `config::test_config_env_lock()`, `ALLOWED_TYPES`) still exist.

## Agents group by a project token, blocked agents lead, and agent views clear from the TUI (PR [#86](https://github.com/cameronsjo/herdr/pull/86))

### feat(sidebar): group agents by a pane metadata token

- **PR:** [cameronsjo/herdr#86](https://github.com/cameronsjo/herdr/pull/86)
- **Files:** `src/config/sidebar.rs`, `src/client/shell/agent_sidebar.rs`, `src/client/shell/aggregate_navigation.rs`, `src/client/shell/endpoint_agents.rs`, the `ordered_agent_pane_ids` / `online_agent_targets` callers (`actions.rs`, `endpoint_navigation.rs`, `input.rs`, `mobile.rs`, `sidebar.rs`), `src/server/render_scale_benchmark/fork.rs`, `docs/next/website/src/content/docs/{configuration,socket-api}.mdx`, `docs/next/website/src/data/config-reference.json`
- **Replaces:** Nothing upstream. It extends the fork's own agent grouping (PR #11). `AgentGroupBy` gains `Token(String)`, written `group_by = { token = "project" }`, and the enum is no longer `Copy`. The group key comes from the pane's token, then the same token on the workspace, then the workspace. Token values and workspace ids are separate key variants. Token mode stably gathers each group behind its first member in `ordered_agent_pane_ids`, and again in `aggregate_agent_rows` on machine plus group, so it skips the contiguity scan. Workspace mode keeps its order and scan. `blocked_first` stably moves blocked agents to the front of each run. The ordering functions take `&AgentsSidebarConfig` instead of the sort alone. Upstream edits to `ordered_agent_pane_ids`, `aggregate_agent_rows`, or their callers will conflict here.
- **Regression check:** `cargo nextest run -E 'test(/token_group|blocked_first|grouped|grouping/)'`. Staged breaks, each confirmed red then green on 2026-09-19: forcing `gathers_group_runs` to `false` reddens `token_grouping_gathers_one_header_across_workspaces`; dropping the workspace-token lookup in `agent_group_key` reddens `token_grouping_falls_back_to_the_workspace_token`; disabling the `blocked_first` pass in `ordered_agent_pane_ids` reddens the `blocked_first_*` tests. When workspace grouping is suspended by an interleaving view, `blocked_first` treats the whole list as one run (`blocked_first_order`); `blocked_first_leads_the_whole_list_when_a_view_suspends_workspace_grouping` failed before that fix.
- **Render scaling:** `just bench-render-scale`, client shell composition at 15 panes, median µs: background 348 off / 313 workspace / 322 token+`blocked_first`; active 307 / 309 / 303. The token arm reports no tokens, so every key takes the workspace fallback scan.

### feat(sidebar): clear an agent view from the header, global menu, or palette

- **PR:** [cameronsjo/herdr#86](https://github.com/cameronsjo/herdr/pull/86)
- **Files:** `src/client/shell/agent_sidebar.rs` (`render_agent_panel_header`), `src/client/shell/{actions,global_menu,mouse,render,state}.rs`, `src/input/{keybindings,keybind_help}.rs`, `src/config/{keybinds,model}.rs`, `src/main.rs`, `src/server/client_commands.rs`, `tests/fixtures/endpoint-method-shapes-v1.json`
- **Replaces:** Upstream draws an active view's label in place of the sort toggle and gives it no click target. The fork draws `<label> ✕`, which clears the view on click. It also adds `KeybindAction::ClearAgentView`, which serves the global menu while a view is active, the palette, and an optional `keys.clear_agent_view`. **Endpoint contract:** `agent.view.clear` joins `CLIENT_SHELL_METHODS` and the shape fixture as a new entry, as `workspace.open` did in PR #64; no existing digest changes. An upstream change to `CLIENT_SHELL_METHODS` or the fixture will conflict here, and keep the upstream digests.
- **Also:** `normalize_label` (`src/app/agent_view.rs`) now drops format characters (bidi overrides, zero-width) as well as control characters, the way `crate::label::sanitize_label` does for every other label, since the label now sits beside a click target. `crate::label::is_format_char` became `pub(crate)` for it.
- **Regression check:** `cargo nextest run -E 'test(/clear_mark|global_menu_offers|clears_an_agent_view|advertised_client_shell|view_labels_drop_format/)'`. Restoring the control-only filter reddens `view_labels_drop_format_characters`. Staged breaks: zeroing `hits.agent_view_clear` reddens `an_active_view_shows_a_clear_mark_that_clears_it_on_click`; hiding the menu entry reddens `the_global_menu_offers_clear_agent_view_only_while_a_view_is_active`; removing `agent.view.clear` from `CLIENT_SHELL_METHODS` reddens `the_palette_clears_an_agent_view`, since the endpoint refuses the method.

## The endpoint agent list groups per machine, and the check parses a padded counter (PR [#84](https://github.com/cameronsjo/herdr/pull/84))

### fix: reconcile the endpoint agent list and agent focus with the upstream merge

- **PR:** [cameronsjo/herdr#84](https://github.com/cameronsjo/herdr/pull/84)
- **Files:** `src/client/shell/endpoint_agents.rs`, `src/client/shell/agent_sidebar.rs`, `src/client/shell/mouse.rs`, `src/client/shell/endpoint_navigation.rs`, `src/client/shell/tests/endpoints.rs`, `src/client/shell/tests/text_editing.rs`, `src/client/shell/tests/workspace_navigation.rs`
- **Replaces:** Nothing upstream — this carries the fork's sidebar grouping into the endpoint list, which upstream does not group at all. Two defects. **(1) Grouping was built per endpoint.** A header belongs to the first row of a workspace run, and only the aggregated cross-endpoint draw order knows where the runs start, so building rows endpoint-by-endpoint gave every row its own header. Rows are now built in that aggregated order. The run key carries the machine as well as the workspace, because two endpoints can advertise the same workspace id — without the machine, two machines' rows merge into one header and the gap between them disappears. The run-contiguity check moved to a shared helper rather than a second copy of the scan. **(2) A press released without a drag sent `pane.focus` directly.** The fork defers agent focus to the button up so the same press can grow into a drag-move, but the release then bypassed `focus_or_activate`. Local stays on screen while a remote activation is pending, so a bare `pane.focus` answered the endpoint being handed away from instead of cancelling the handoff. The release now routes through `focus_or_activate` the way an ordinary click does.
- **Collision, 2026-09-17:** two upstream tests new in this range assert behaviour the fork deliberately changed, and both are re-expressed against the fork with the reason inline. `client::shell::tests::text_editing::all_naming_targets_preserve_submission_and_empty_semantics` and `client::shell::tests::workspace_navigation::foreign_preview_blocks_keyboard_actions_but_keeps_active_action_context` both expect `workspace.create` for an unnamed new space; the fork's `new_workspace_method` (fork PR [#64](https://github.com/cameronsjo/herdr/pull/64)) submits `workspace.open` instead, focusing a space already on the path rather than stacking a second one on it. That divergence has its own section below, *Two spaces on one path stay independent* — backfilled 2026-09-18, because the gap is what made this collision unexplained when the sync hit it. `client::shell::tests::endpoints::local_agent_click_can_cancel_a_pending_remote_switch` expects the cancelling action on the button down; under the fork's drag-move it lands on the button up, so the test now sends the press and the release.
- **Regression check:** `cargo nextest run -E "test(/client::shell::/) or test(/mouse/) or test(/endpoint/) or test(/sidebar/)"` — 655 tests, all passing. Staged-break checks: passing `grouped: false` in `endpoint_agents.rs` must redden `grouped_agents_keep_headers_and_gaps_separate_for_identical_workspace_ids`; dropping the machine from the run key must redden the same test's gap assertions while leaving its header assertions green; restoring the bare `pane.focus` in `mouse.rs` must redden `local_agent_click_can_cancel_a_pending_remote_switch`.
- **Render scaling:** `just bench-render-scale`, required by CLAUDE.md for any change that widens work in a render-scaled loop. The `fork agent grouping` scenarios compare grouping off against grouping on at 1 and 15 panes; grouping on is not the slower arm in any of the four rows. Background, median µs at 15 panes: 235 off / 234 on (1.12x and 1.11x of the 1-pane median). Active: 220 off / 218 on (1.05x both). The run-contiguity check is `O(runs x n)` in the worst case — many single-agent workspaces — so it is not free by construction; these are the measured numbers at the shapes the bench covers, and the endpoint list at high endpoint counts is not among them.

### test(ci): point the publishing-boundary test at the fork's own release pipeline

- **PR:** [cameronsjo/herdr#84](https://github.com/cameronsjo/herdr/pull/84)
- **Files:** `scripts/release-workflows.test.ts`
- **Replaces:** Upstream's `scripts/release-workflows.test.ts`, new in this range. It loads `release.yml` and reads `jobs["validate-release-source"].steps[0]` at module scope, which throws before any test runs: the fork replaced `release.yml` with its own build/release/tap-bump pipeline (ledger PR #3/#4) and has no such job. The gate step itself is still reachable — upstream anchors it in `preview.yml`, which the fork does not modify — so the admin-gate assertions and the fail-closed behaviour test are kept, sourced from there. The release half is retargeted at what the fork actually ships: the tag trigger is the fork's `v*-palette.[0-9]*` namespace rather than upstream's `v*`, and in place of a multi-admin gate the fork's boundary is least privilege, so the test now pins the workflow default of `contents: read`, each job's own widening, and the tap App token's scoping to `homebrew-tap` with `permission-contents: write`. The fork has one owner; it does not have, and this does not remove, a two-actor gate.
- **Regression check:** `bun test scripts/release-workflows.test.ts` — 6 tests, 40 assertions. Staged-break check: changing `repositories: homebrew-tap` to any other repository in `release.yml` must redden `the fork's release pipeline grants the narrowest permissions it can`, and the file restored must return it to green. Confirmed both directions 2026-09-18. This test runs under `just maintenance-test`, which `scripts/docker-check.sh` does **not** invoke — the container gate covers `cargo fmt`, `clippy` and `nextest` only, so CI caught this after a green local gate.

### fix(ci): match the check's failure parser to nextest's padded counter

- **PR:** [cameronsjo/herdr#84](https://github.com/cameronsjo/herdr/pull/84)
- **Files:** `scripts/docker-check.sh`, `scripts/test_docker_check.py`
- **Replaces:** Nothing upstream — `docker-check.sh` is fork-only. `classify_nextest_failures` read the binary id from awk field 5 and the test name from everything after it. nextest right-aligns its progress counter once the suite is large enough (`(  69/3862)` rather than `(1/1)`), and that padding splits the counter across two whitespace-delimited fields, shifting every field after it. The suite crossed that threshold in this sync, so the extracted names stopped matching `KNOWN_ENV_FAILURES` and eighteen known-environment failures were reported as unexpected regressions — each one also paying for a pointless isolated retry. The prefix is now matched by shape and stripped, so the binary id and test name do not depend on the counter's width. Eight entries were added to the allowlist (the hook, process-cleanup and letta tests), each reproduced on a detached worktree of pristine `origin/master` in the same image, which is the bar the script's own comment sets for extending it; this reverses the older note there that process-cleanup tests "run normally".
- **Regression check:** `python3 -m unittest scripts.test_docker_check` — 10 tests. The two new ones (`test_padded_progress_counter_still_matches_the_allowlist`, `test_padded_progress_counter_extracts_binary_and_name`) were confirmed red against the old parser before the fix; the pre-existing eight all used an unpadded `(1/1)` line and stayed green through the defect, which is why it shipped. `shellcheck scripts/docker-check.sh` reports only a pre-existing SC1091 info on `source "$HOME/.cargo/env"`.

## The fork's Zig toolchain follows libghostty-vt to 0.16.0

### ci: install Zig 0.16.0 for the macOS builds and the container check

- **Files:** `.github/workflows/release.yml`, `.github/workflows/build-artifacts-manual.yml`, `scripts/docker-check/Dockerfile`, `scripts/docker-check.sh`
- **Replaces:** Nothing upstream — this keeps the fork's own three Zig installs in step with the vendored library. The 2026-09-17 sync raised `vendor/libghostty-vt/build.zig.zon`'s `minimum_zig_version` to `0.16.0`, and all three fork-owned installs still pinned 0.15.2, so every one of them fails in `build.rs` before a line of Rust compiles. Upstream moved to `vercel-labs/setup-zig` at 0.16.0; the fork's two macOS workflows stay on Homebrew (`brew install zig`, now 0.16.0) because the reason for that divergence is unchanged — a stock Zig tarball's bundled libSystem predates the macOS 26 SDK and fails to link, while Homebrew's zig is built against the runner's own SDK. Two related things follow. The cache key moves to `homebrew-zig-0.16-`, since a restored 0.15 download is worthless. And `docker-check.sh` gains a bounded-parallelism prebuild step: Zig 0.16 compiles libghostty-vt's SIMD and wuffs C++ sources one job per visible CPU, and four at once exceed a 4 GiB Docker VM, so the kernel kills the compile and `build.rs` reports it as a plain zig-build failure naming the Zig version — the symptom points at the toolchain, not at memory. The prebuild warms `.zig-cache` under `ZIG_BUILD_CPUSET` (default `0-1`) and the main run keeps every CPU for nextest. The same ceiling has a second offender: four concurrent `rustc` processes on this crate overrun the VM too, and the kill arrives as `could not compile \`herdr\` ... (signal: 9, SIGKILL: kill)` with no diagnostic attached, which reads as a compiler crash. The main run now sets `CARGO_BUILD_JOBS` (default `2`); nextest still runs its tests at full width, since those are small.
- **Regression check:** `bash scripts/docker-check.sh` reaches nextest rather than failing in `build.rs`; `shellcheck scripts/docker-check.sh` and `actionlint .github/workflows/release.yml .github/workflows/build-artifacts-manual.yml` exit 0 (the manual workflow carries one pre-existing SC2086 info on an unquoted `$GITHUB_ENV`, unrelated to this change); `python3 -m unittest scripts.test_docker_check` passes. Staged-break check: setting `ARG ZIG_VERSION=0.15.2` back in the Dockerfile must fail with `Building Herdr requires Zig 0.16.0`.

## The palette live check refuses a stale binary and an unknown plugin id (PR [#83](https://github.com/cameronsjo/herdr/pull/83))

### fix(scripts): refuse a stale binary and an unknown plugin id in the palette live check

- **PR:** [cameronsjo/herdr#83](https://github.com/cameronsjo/herdr/pull/83)
- **Files:** `scripts/palette-live-check.sh`
- **Replaces:** Nothing upstream — this hardens a fork-only script added by PR [#61](https://github.com/cameronsjo/herdr/pull/61). Three defects, all one failure mode: a step that shows the same result whether the feature works or not. **(1) Staleness.** The script tested the binary with `-x` alone, which asks whether a file exists and never whether it matches the source. A binary built before #61 draws the pre-change palette — directional leaf rows, `unset` in the key column, the untrimmed footer — so every step reports old behaviour and the operator concludes the feature is broken. Measured 2026-09-15: a two-day-stale binary produced exactly that reading and cost an evening. It now compares the binary against `palette.rs`, `overlays.rs`, and `palette/input.rs`, and refuses when a listed source is missing rather than silently skipping it. **(2) Wrong plugin id.** The override marked `collie:uninstall`, but `action_is_marked_destructive` (`src/client/shell/palette.rs`) joins `"{plugin_id}:{action_id}"` and compares exact strings with no normalization, and Collie's id is `herdr.collie`. The palette row displays `Collie`; the display name is not the id. **(3) No plugin registry at all — the root defect.** The script launches the debug binary under a throwaway `mktemp` config dir, and `registry_path()` is `config_dir().join("plugins.json")` while `app_dir_name()` is `herdr-dev` under `debug_assertions`. That path never existed in the throwaway, so the run saw **zero** plugins and step 5 had no row to tag at all — it had been untestable since the script was written. Measured 2026-09-15: unseeded prints `No plugins installed`; seeded with the operator's real registry prints all five. The script now copies that registry in (absolute `plugin_root` paths make a copy sufficient, and the real config is only ever read) and asks the binary under test which plugins it can see, rather than checking a directory on disk that this run does not read. This matters because three regression checks in this ledger invoke the script and the repo wires no shell linting in CI, so nothing else guards it.
- **Regression check:** `shellcheck scripts/palette-live-check.sh` exits 0. Staged-break checks, each of which must refuse and then pass again on an unmodified tree: `touch src/client/shell/palette.rs` must refuse with `Stale binary:`; changing the override id to `collie:uninstall` must refuse with `no plugin with the id`; and removing the `plugins.json` copy must make the same guard refuse, since an unseeded run genuinely has no plugins. The positive direction matters equally — with the correct `herdr.collie:uninstall` and the registry seeded, the guard must pass through and step 5 must show the `[destructive]` tag.

## macOS CI row is gated to pushes on master (PR [#82](https://github.com/cameronsjo/herdr/pull/82))

### ci: run the macOS matrix row only on a push to master

- **PR:** [cameronsjo/herdr#82](https://github.com/cameronsjo/herdr/pull/82)
- **Files:** `.github/workflows/ci.yml`
- **Replaces:** Upstream's `check` job carries a static `strategy.matrix.include` holding an `ubuntu-latest` and a `macos-latest` row, so both run on every pull request and every push. macOS bills at 10x Linux, and the fork measured 116 CI runs over 2026-09-01..15 — 81 of them pull requests, each paying for a macOS leg. The fork selects the row list by expression instead, `fromJSON` over `github.event_name == 'push' && github.ref_name == 'master'`, because `matrix` is not an available context for a job-level `if:` — the same limitation that led the `windows-latest` row to be deleted rather than gated. Pull requests, and pushes to the `windows` branch, now run `ubuntu-latest` alone. Two consequences are recorded in the workflow at the point of change rather than only here. First, `check (macos-latest)` no longer reports on a pull request, which is safe only while this repo has no branch protection and no ruleset (verified against the API, not assumed); add either and the row must be restored or its context made optional. Second, macOS drops from two runs per commit to one, and the pre-existing `concurrency` block cancels an in-flight master run, so merging two PRs in quick succession can leave a commit with no macOS result recorded — which matters because `release.yml` builds `aarch64-apple-darwin` and macOS is the only platform this fork ships. No tag was ever gated on CI, before or after: `release.yml` triggers on the tag push alone with no `needs:` on the `check` job.
- **Regression check:** `actionlint .github/workflows/ci.yml` exits 0; and on a pull request the run's check list is `check (ubuntu-latest)` plus `conventional-commits` with no macOS leg present — PR #82's own checks are the measurement.

## Positional workspace id fallback is bounded to the live list (issue [#77](https://github.com/cameronsjo/herdr/issues/77))

### fix: drop pane aliases when their pane closes, and the #99 review follow-ups

- **PR:** see the PR that adds this entry ([#100](https://github.com/cameronsjo/herdr/issues/100))
- **Files:** `src/app/actions.rs` (`remove_plugin_pane_records`), `src/app/ids.rs` (doc comment only), `src/app/api/workspaces.rs` (new test), `src/server/headless/tests/mod.rs` (socket dir name)
- **Replaces:** Upstream evicts a pane's raw and public id aliases only in `handle_pane_died`, which returns before that cleanup when the pane's tab or workspace is already gone. So closing a tab or workspace that held a moved pane left an alias pointing at a dead pane. A lookup through it finds no pane, so nothing misroutes (pane ids are never reused), but the entry leaks and `assert_invariants_for_test` rejects the state. Every close path calls `remove_plugin_pane_records` with the closed panes, so the fork prunes both alias maps there. Three items from the #99 review are folded in. The `public_workspace_id` doc comment now says its empty-id fallback only keeps that one call from panicking: callers like `workspace_info` index the same `ws_idx` next to it, and the real protection is `parse_workspace_id`'s bound. A seeded random-sequence test runs 24 seeds × 40 steps of add, switch, select, move, close, merge, tab and pane close, and split, checking `assert_invariants_for_test` after every step, so a sequence that leaves `active` or `selected` out of range now fails with its seed and step. It found the alias leak; no stale `active`/`selected` was found. The headless harness's socket dir gains an atomic counter, so two tests in one process can no longer collide on a pid-plus-clock name.
- **Regression check:** `cargo test --bin herdr random_close_move_merge_sequences` and `cargo test --bin herdr positional_workspace_ids` (plain `cargo test`, now safe). Staged break, 2026-10-12: removing the two alias `retain` lines in `remove_plugin_pane_records` reddens the random-sequence test at seed 4, step 36 (`public pane alias ... references missing pane`).

### fix(api): bound the positional workspace id fallback to the live list

- **PR:** [cameronsjo/herdr#81](https://github.com/cameronsjo/herdr/pull/81)
- **Files:** `src/app/ids.rs`, `src/app/api/workspaces.rs`, `src/app/api/tabs.rs`, `src/app/api/panes.rs`
- **Replaces:** Upstream's `parse_workspace_id` accepts `w_N` and a bare `N` as a documented positional convenience (the Nth workspace in display order) but returns the parsed index without checking it against the live workspace list, so `"999999"` resolves to `Some(999998)`. Three call sites re-check `< len` by hand; the ones that do not index `self.state.workspaces` directly through `public_workspace_id`, so one socket request panics the whole server — reachable through `pane.move` with `destination.type = "new_tab"` (the hand-rolled guard further down that arm is dead code, the panic fires first) and through `Method::WorkspaceFocus` in the headless client-view path, which has no guard at all. The fork adds one `.filter(|index| *index < len)` to the parser, so every caller gets `None` for an out-of-range positional id and maps it to the `workspace_not_found` error it already returns for an unknown id. The positional convenience itself is unchanged and still documented. The three hand-rolled `< len` guards are kept deliberately as defense in depth at their indexing calls; only their now-false comments are corrected.
- **Regression check:** `cargo nextest run --locked -E 'test(positional_workspace_id) + test(api_pane_move_to_new_tab_rejects)'`.
- **Retired 2026-10-20 for the parser filter:** upstream fixed the same panic with the same filter (#5171), which the 2026-10-12 sync merged. That sync kept the fork's line and comment; `parse_workspace_id` now carries upstream's text, so the parser no longer diverges. The fork's request-path tests and the three hand-rolled `< len` guards stay, and their regression check above still applies.

### test: pin out-of-range positional workspace ids on every request path

- **PR:** [cameronsjo/herdr#99](https://github.com/cameronsjo/herdr/pull/99)
- **Files:** new fork file `src/server/headless/tests/fork_workspace_ids.rs`; one `mod` line in `src/server/headless/tests/mod.rs`
- **Replaces:** Nothing; test only. A live probe on 2026-10-06 sent 518 bad-id requests (25 workspace-id methods × 16 ids, plus tab and pane ids that embed a workspace id) to isolated servers. Stock 0.9.3 died 18 times, through `pane.move` to a new tab (its `src/app/ids.rs:16`, the bare index in stock's `public_workspace_id`) and through `worktree.list` and `worktree.open` (`src/app/api/worktrees/reads.rs:123`, a site #77 did not name). The fork release `v0.9.1-palette.1` and `master` `704f48aa` died 0 times. The test sends `workspace.focus`, `worktree.list`, `worktree.open`, and `pane.move` over both the public socket and an attached client's shell. Two of those sites had no test before: the worktree reads and the client-shell `workspace.focus`, which only an attached TUI client reaches. A second test is the positive control: in-range `N` and `w_N` still focus on both paths, each request starting from the other workspace so a pass needs it to move focus. It checks the end result only: the client-shell navigation arm is one of two places that move the client, and disabling it alone leaves the test green.
- **Regression check:** `cargo nextest run -E 'test(positional_workspace_ids)'`. Plain `cargo test` is safe since #100 added a counter to the harness's socket dir name. Staged breaks, 2026-10-06: removing the `.filter` in `parse_workspace_id` reddens the negative test at `reads.rs:123`, and with its loop narrowed to the client-shell `workspace.focus`, at the `public_workspace_id` `debug_assert!` (test builds only; a release build logs and returns an empty id there); a filter that rejects every id reddens the positive control.

## Keyboard and palette workspace reorder respects worktree grouping (issue [#48](https://github.com/cameronsjo/herdr/issues/48))

### fix(client): move a worktree group as a block from the keyboard and palette

- **PR:** [cameronsjo/herdr#79](https://github.com/cameronsjo/herdr/pull/79)
- **Files:** `src/client/shell/actions.rs`, `src/client/shell/context_menu.rs`, `src/client/shell/tests/keybindings_settings.rs`
- **Replaces:** Upstream's keyboard reorder actions (`MoveWorkspacePrevious`/`MoveWorkspaceNext`, which the command palette also dispatches) index the raw `snapshot.workspaces` list and emit a plain `WorkspaceMove`. The sidebar drag path and the workspace context menu both go through `workspace_move_method`, which positions over group roots only, moves a worktree group as one `WorkspaceMoveBlock`, and refuses a linked worktree outright. So the same one-slot reorder behaved three different ways: from the keyboard a group root left its own block behind, and a linked worktree child reordered even though neither of the other two paths would move it. The fork widens `workspace_reorder_method` to `pub(super)` and routes the keyboard arm through it, so the context menu, the keyboard, and the palette share one builder. User-visible removal: keyboard reorder of a linked worktree child is now a no-op, matching drag and the context menu.
- **Regression check:** `cargo nextest run --locked -E 'test(workspace_reorder)'` — the three grouped cases plus the unchanged plain-list wrapping test.

## Merge stops reporting moved tabs as closed (issue [#53](https://github.com/cameronsjo/herdr/issues/53))

### fix(api): stop workspace.merge reporting moved tabs as closed

- **PR:** [cameronsjo/herdr#78](https://github.com/cameronsjo/herdr/pull/78)
- **Files:** `src/app/api/workspaces.rs`, `src/app/api/plugins/context.rs`, `docs/next/website/src/content/docs/socket-api.mdx`
- **Replaces:** Upstream emits merge's `workspace.closed` with a pre-drain `WorkspaceInfo`, snapshotted before the source's tabs leave. That snapshot asserts tab and pane counts and an `active_tab_id` for content that moved to the target and is alive, so the event describes a loss that did not happen. The only in-repo consumer, `plugin_context_for_event`, then turns that `active_tab_id` into a plugin hook's `tab_id`, naming a tab that is open in the target. The fork collects ids only and emits `workspace: None`, matching `pane.move`, which already closes a drained workspace that way. The `TabMovedAcrossWorkspaces` events emitted just before carry every moved tab's source workspace, previous id, and new id, so no information is lost.
- **Regression check:** `cargo nextest run --locked -E 'test(api_workspace_merge)'`.

## Two spaces on one path stay independent, and a new space opens rather than duplicates (PR [#64](https://github.com/cameronsjo/herdr/pull/64))

Backfilled 2026-09-18. This divergence landed 2026-09-12 and was never given a ledger section, which is how the 2026-09-17 sync met it as an unexplained test collision.

### feat(api): add workspace.open, which focuses a space already on the path

- **PR:** [cameronsjo/herdr#64](https://github.com/cameronsjo/herdr/pull/64)
- **Files:** `src/api/schema/workspaces.rs`, `src/api/schema.rs`, `src/api/schema/response.rs`, `src/api/mod.rs`, `src/api/server.rs`, `src/app/api/workspaces.rs`, `src/app/actions.rs`, `src/cli/workspace.rs`, `src/cli/runtime.rs`, `src/cli/spec.rs`, `src/client/shell/actions.rs`, `src/client/shell/overlay_input.rs`, `src/server/client_commands.rs`, `src/server/headless/client_views.rs`, `tests/fixtures/endpoint-method-shapes-v1.json`
- **Replaces:** Nothing upstream — upstream has only `workspace.create`, which creates unconditionally, so the new-space action stacked a second space on a checkout that already had one. The fork adds `workspace.open`: it focuses the space already on a path and creates one only when the path has none. `workspace.create` is left creating unconditionally on purpose, rather than gaining a reuse flag, because an older server that ignored such a flag would report success while opening the very duplicate the caller asked it to avoid. A new method name cannot be silently ignored. `ClientShellState::new_workspace_method` picks between them and falls back to `workspace.create` when the server does not advertise `workspace.open`, which keeps the stable endpoint contract intact. Typing a name in the new-space prompt is a deliberate second space and still creates — so the choice is made on whether a label was typed, not on the surface the action came from.
- **Collision, 2026-09-17:** upstream added two tests in that sync's range which assert `workspace.create` for an unnamed new space — `client::shell::tests::text_editing::all_naming_targets_preserve_submission_and_empty_semantics` and `client::shell::tests::workspace_navigation::foreign_preview_blocks_keyboard_actions_but_keeps_active_action_context`. Both are re-expressed against `workspace.open` in PR [#84](https://github.com/cameronsjo/herdr/pull/84). Expect them to conflict again on any sync that touches either file.
- **Regression check:** `cargo nextest run --locked -E 'test(workspace_open_focuses_the_space_already_on_the_path) + test(workspace_open_reuse_prefers_focused_space_over_lowest_index) + test(public_workspace_open_focuses_every_client_on_a_reused_space) + test(advertised_client_shell_method_shapes_stay_at_the_v1_contract)'`. The last one is the endpoint contract guard: it pins the advertised method's wire shape in `tests/fixtures/endpoint-method-shapes-v1.json` and refuses to rewrite an existing digest, so a later change to `WorkspaceOpenParams` cannot quietly reshape a published method.

### fix(client): scope a worktree group to its own linked spaces

- **PR:** [cameronsjo/herdr#64](https://github.com/cameronsjo/herdr/pull/64)
- **Files:** `src/client/shell/sidebar.rs`, `src/client/shell/endpoint_sidebar.rs`, `src/client/shell/mouse.rs`, `src/app/api/worktrees.rs`, `src/server/client_commands.rs`
- **Replaces:** Upstream keys a worktree group by repo, so a second space on the same checkout shares the key and renders as a group child. Collapsing the group then dropped that row entirely, leaving the space with no hit rect — unreachable by click and unable to receive a dragged tab or pane. Group scope now covers the first non-linked space plus its linked worktrees only, applied consistently in the sidebar, the block move, the merge and close dialogs, and the server's own close indices, so a duplicate never hides inside a group and a group close never takes one with it. Worktree parent lookups prefer the focused space over the lowest matching index, which matters once two spaces can match.
- **Regression check:** `cargo nextest run --locked -E 'test(/client::shell::tests::agents_worktrees_notifications::/)'`, plus `just bench-render-scale` for the sidebar grouping rework, measured at 1, 15 and 50 workspaces: 1.00x / 0.92x / 1.19x median client-shell composition. The 50-workspace row is a real 19% rise, not noise to wave past; it was accepted because the shape is sublinear in workspace count, not because the number is flat.

## Keep SIGPIPE ignored inside the unit-test harness (issue [#39](https://github.com/cameronsjo/herdr/issues/39))

### fix(platform): keep sigpipe ignored inside the unit-test harness

- **PR:** [cameronsjo/herdr#75](https://github.com/cameronsjo/herdr/pull/75)
- **Files:** `src/platform/unix_common.rs`
- **Replaces:** The issue's premise — a pty write losing its reader — was wrong. `begin_cli_output()` flips SIGPIPE to `SIG_DFL` so a piped `herdr --help | head` dies quietly on purpose; correct for the shipped CLI. `cargo test --bin herdr` runs every test in one process, so once a test that prints and then drops a reading peer calls it, SIGPIPE stays fatal for every later write-to-closed-pipe in the same run, and the suite exits 141 partway through. Which test dies moves between runs because the process-wide disposition, not any one test, is what changed. `cargo nextest run` hides this because it isolates each test in its own process. Restoring the disposition inside just the one offending test was rejected: it silently re-breaks the next time any test exercises a CLI print path. The fix gates `begin_cli_output`/`end_cli_output`'s bodies on `#[cfg(not(test))]` so the test harness never touches the process-wide disposition.
- **Regression check:** `cargo test --bin herdr > log 2>&1; echo $?` — prints a `test result:` line and no longer exits 141.

## Cold restore keeps the agent name as a label ([#35](https://github.com/cameronsjo/herdr/issues/35))

### fix: keep a cold-restored agent name as a label and log the dropped routing key

- **PR:** [cameronsjo/herdr#76](https://github.com/cameronsjo/herdr/pull/76)
- **Files:** `src/persist/restore.rs`
- **Replaces:** Upstream drops a stored agent name silently when a pane restores through a fresh shell with no agent, which is the normal path under `resume_agents_on_restore = false`. Dropping the name is correct — `AppState::resolve_agent_target` matches on it with no liveness check, so a restored name would route `agent prompt` into an interactive shell — but a pane named only through `agent.start` comes back with no name and no label, because `agent.start` and `agent.rename` never set a manual label. The fork logs the dropped name at debug and carries it into the manual label when the pane came back with no label of its own.
- **Regression check:** `cargo nextest run --locked -E 'test(cold_restore_drops_a_managed_agent_name_and_keeps_it_as_a_label) + test(cold_restore_carries_the_agent_name_over_a_blank_stored_label)'`.

## Split server_not_running into three states

### fix(cli): split server_not_running into three states so a wedged api socket is not misdiagnosed

- **PR:** [cameronsjo/herdr#70](https://github.com/cameronsjo/herdr/pull/70)
- **Files:** `src/cli/server_not_running.rs`, `src/cli.rs`, `src/server/autodetect.rs`, `src/api/client.rs`, `src/cli/status.rs`, `tests/cli/sessions.rs`
- **Replaces:** Upstream reports the single code `server_not_running` for three distinct states: (a) no socket at all, (b) a stale socket file left by a crashed server, and (c) a live api socket that refuses connections while the server itself is still up (its api accept loop died) — for which the existing remedy, running `herdr`, is actively harmful because it deletes the refused socket and starts a second server. The fork adds `server_api_not_accepting` (api socket refuses, but the paired client socket still answers — discriminated via the existing `is_server_listening_at` probe against the derived client socket) and `server_api_not_responding` (the socket accepts but the handshake ping never replies — discriminated with a new bounded `ApiClient::status_with_timeout`, 10s, so a merely-slow server is not misdiagnosed against the server's own 5s `INITIAL_REQUEST_TIMEOUT`). Case (b), stale socket + no live client socket, is unchanged and still reports `server_not_running` with the original `run \`herdr\`` remedy. `herdr status`/`herdr status server` gain a matching `NotAccepting` runtime-status variant (`status: running but not accepting api connections`, `running: false` in JSON so existing scripts stay correct). No wire-protocol impact: the codes are CLI-local strings, never cross the network. On Windows the wedged case still falls back to the pre-existing message, because `is_server_listening_at`'s Windows branch already probes through the api socket itself; the handshake-timeout code path works identically on every platform.
- **Regression check:** `just test-one dead_server` (3 passed, up from 2), `just test-one classifier_ignores_unrelated_io_kinds`, `just test-one status_with_timeout`, `just test-one server_status`, `just lint`.
- **Collision, 2026-09-17:** upstream has since added its own `ApiClient::status_with_timeout`, built on `request_value_with_timeout` (socket timeouts). The sync merged both definitions into the same `impl` and the crate stopped compiling. The fork's implementation wins and is now `pub`: `set_timeout_best_effort` is a no-op on Windows named pipes, so a socket-timeout version leaves the wedged case unbounded there, which is exactly the case this guard exists for. `request_value_with_timeout` keeps its other caller in `src/api/status.rs`. Upstream also added `cli::target::server_status`, which rediscovers a stale SSH bridge and retries; the fork's bounded probe now lives inside that function's local branch rather than beside it, so `herdr status` and the handshake gate get both behaviours.

## Fix flaky plugin-path capture tests (issue #38)

### fix(tests): publish plugin capture files by atomic rename

- **PR:** [cameronsjo/herdr#68](https://github.com/cameronsjo/herdr/pull/68)
- **Files:** `src/app/api/plugins/mod.rs`
- **Replaces:** Every test capture in this file redirected a shell `printf` straight into its destination file (`> {capture}`), so a reader could observe the file between the shell's truncating `>` open and the command's writes landing — a partial-read race, not a config-dir race. `plugin_pane_open_injects_plugin_paths_and_protects_overrides` was the one that hit it (~1.5% flake rate; a probe of 300 runs of the old shape read a partial line 20 times). Every capture writer now redirects to a sibling `.tmp` path and renames it into place, matching the pattern the Windows sibling test already used, so `read_capture_when_ready`'s first non-empty read is always a complete one. Also serializes every test in this file that resolves `config_dir()`/`state_dir()` — the two plugin-path tests plus `plugin_link_creates_stable_config_and_state_dirs` and `plugin_link_seeds_stable_config_dir_from_legacy_unhashed_dir` — against `non_cli_plugin_consumers_refresh_global_enabled_state`'s `XDG_CONFIG_HOME` mutation via the existing `test_config_env_lock`, a real (if unrelated) race under plain `cargo test`. That mutating test now restores `XDG_CONFIG_HOME` from a `Drop` guard, so a failed assertion inside it can no longer leak a temp config home into every later test.
- **Regression check:** `python3` probe comparing the old and new writer shape 300 times each (old: 20/300 partial reads; new: 0/300) — see the plan for the script. Also `just test-one 'app::api::plugins::tests::'`.

## Palette direction chooser, destructive confirm, and review findings (`docs/plans/2026-09-09-palette-direction-chooser-and-review-findings.md`)

### feat(client): collapse move and swap rows into a direction chooser

- **PR:** [cameronsjo/herdr#61](https://github.com/cameronsjo/herdr/pull/61)
- **Files:** `src/client/shell/{palette.rs,palette/input.rs,overlay_input.rs,overlays.rs,state.rs,mouse.rs,composition.rs,input_source.rs}`
- **Replaces:** Upstream lists every directional action as its own palette row, so a query for `move` returns nine rows. The fork collapses `move tab`, `move workspace`, and `swap pane` into one row each, opening a generic chooser overlay that also replaced upstream's two-button split-into-tab picker. Leaf rows stay reachable by naming a direction.
- **Regression check:** `bash scripts/palette-live-check.sh` from a terminal, then steps 1-3 it prints.

### feat(plugins): tag and confirm destructive palette actions

- **PR:** [cameronsjo/herdr#61](https://github.com/cameronsjo/herdr/pull/61)
- **Files:** `src/api/schema/plugins.rs`, `src/app/api/plugins/manifest.rs`, `src/config/{model.rs,io.rs}`, `src/client/shell/{config.rs,state.rs,palette.rs,palette/input.rs,overlays.rs}`, `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/{plugins,configuration}.mdx`, `docs/next/website/src/data/config-reference.json`
- **Replaces:** Upstream runs any plugin action straight from the palette with no marking and no confirm. The fork adds an optional `destructive` key to `[[actions]]` in the plugin manifest and a `[palette] destructive_actions` config override for third-party plugins; a marked row is tagged and asks before running, with cancel selected by default. The manifest field is additive and optional, so an older manifest parses unchanged.
- **Regression check:** `cargo nextest run -E 'test(destructive)'` — nine tests. Also `python3 scripts/config_reference_check.py` exits 0, and step 5 of `bash scripts/palette-live-check.sh`.

### fix(client): palette right column shows a key, a match reason, or a dash

- **PR:** [cameronsjo/herdr#61](https://github.com/cameronsjo/herdr/pull/61)
- **Files:** `src/input/keybind_help.rs`, `src/client/shell/{palette.rs,overlays.rs}`
- **Replaces:** Upstream leaves the palette's right column empty for a row with no key binding, while the keybind help screen shows the word `unset` for the same fact. The fork makes `KeybindHelpEntry.key` an `Option`, so the palette can show the key, else the keyword that matched the row, else a dash — and the help screen keeps `unset` at its own render site. Upstream's mid-word substring match tier is also gone: it is what put a plugin's `(remove service)` action in front of a query for `move`.
- **Regression check:** step 4 of `bash scripts/palette-live-check.sh`.

## Land the upstream sync, cut the release, fit CI to the fork (`docs/plans/2026-09-08-land-the-upstream-sync-cut-the-release-fit-ci-to-the-fork.md`)

### ci: fit the pipeline to what this fork actually ships

- **PR:** [cameronsjo/herdr#59](https://github.com/cameronsjo/herdr/pull/59)
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/nix.yml`, `.github/workflows/release.yml`, `.github/workflows/sync-upstream.yml`, `.github/workflows/windows-arm64.yml`, `README.md`, `justfile`, `scripts/fork_release_notes.py`
- **Replaces:** Upstream's CI matrix builds and gates targets (Nix, Windows ARM64, the upstream-only release/sync jobs) this fork does not ship or need.
- **Regression check:** `gh run list --workflow ci.yml -R cameronsjo/herdr --limit 1` shows a green run on the latest `master` push.

### chore: script the fork release cut

- **PR:** [cameronsjo/herdr#58](https://github.com/cameronsjo/herdr/pull/58)
- **Files:** `scripts/cut-fork-release.sh`
- **Replaces:** Upstream has no fork-suffixed release process; this scripts the tag-and-push sequence the fork's `vX.Y.Z-palette.N` tags need.
- **Regression check:** `bash scripts/cut-fork-release.sh --dry-run v0.9.0-palette.1` prints the tag-and-push sequence without running it. The tag argument is required; the bare form tags and pushes with no prompt.

## Sync upstream into the fork (`docs/plans/2026-09-07-sync-upstream.md`)

No section of its own — this plan's sole output was the `#56` upstream-sync
merge, listed above under Upstream syncs. The plan also preserved fork
behavior across the merge: fork README, palette, grouped sidebar and
trailing tokens, workspace merge/move operations, integration behavior, and
fork-only distribution gates — see each owning section below for its
regression check.

## herdr fork: land the queue, sync upstream, close the issues, fix reshuffle (`docs/plans/2026-09-04-herdr-fork-land-the-queue-sync-upstream-close-the-issues-fix.md`)

### feat(api): add workspace.merge behind the group-close intent gate

- **PR:** [cameronsjo/herdr#50](https://github.com/cameronsjo/herdr/pull/50)
- **Files:** `docs/next/api/herdr-api.schema.json`, `src/cli/*`, `docs/next/website/src/content/docs/{cli-reference,socket-api}.mdx` (en/ja/zh-cn), `docs/next/website/src/data/config-reference.json`
- **Replaces:** Upstream has no `workspace.merge` verb; this fork adds it gated behind an explicit group-close intent to avoid silent data loss on merge.
- **Regression check:** `herdr workspace merge --help` lists the command; `cargo nextest run -E 'test(/workspace_merge|merge_workspace/)'` (13 tests; the old `cargo test workspace::merge` matched none).

### feat(mouse): reach every move and reorder from the menus and the mouse

- **PR:** [cameronsjo/herdr#47](https://github.com/cameronsjo/herdr/pull/47)
- **Files:** `src/client/shell/{actions,composition,context_menu,mouse,render}.rs`, `docs/next/website/src/content/docs/concepts.mdx` (en/ja/zh-cn)
- **Replaces:** Upstream's workspace/pane move and reorder actions are keyboard-only; this fork adds mouse and context-menu paths to the same actions ("reshuffle-b").
- **Regression check:** drag a pane between tabs in the client and confirm the context menu offers move/reorder.

### feat(keys): reach every workspace and pane move from keyboard, palette and CLI

- **PR:** [cameronsjo/herdr#45](https://github.com/cameronsjo/herdr/pull/45)
- **Files:** `src/cli/{runtime,spec,workspace}.rs`, `src/client/shell/actions.rs`, `docs/next/website/src/content/docs/cli-reference.mdx` (en/ja/zh-cn), `docs/next/website/src/data/config-reference.json`
- **Replaces:** Upstream exposes only a subset of workspace/pane moves through the CLI and palette ("reshuffle-a").
- **Regression check:** `herdr workspace move --help` and `herdr pane move --help` both list every documented subcommand; palette search for "move" surfaces all of them.

### feat: improve Codex lifecycle and pane titles

- **PR:** [cameronsjo/herdr#26](https://github.com/cameronsjo/herdr/pull/26)
- **Files:** `src/agent_resume.rs`, `src/app/actions.rs`, `src/integration/{mod.rs,config_edit.rs}`, `src/integration/assets/codex/herdr-agent-state.{sh,ps1}`, `docs/next/website/src/content/docs/{agents,integrations}.mdx`
- **Replaces:** Upstream's Codex integration lacks the fork's pane-title and lifecycle-resume handling.
- **Regression check:** launch a Codex agent pane and confirm the title reflects live state; `cargo nextest run -E 'test(/integration::tests::.*codex|agent_resume::tests::codex|codex_hook/)'` (15 tests; the old `cargo test integration::codex` matched none).

### fix(cli): register live-handoff in the server command spec

- **PR:** [cameronsjo/herdr#30](https://github.com/cameronsjo/herdr/pull/30) (issue #24)
- **Files:** `src/cli/{completion,spec}.rs`
- **Replaces:** Upstream's `server` command spec omits `live-handoff`, so it was invisible to `--help` and shell completion despite being runnable.
- **Regression check:** `herdr server --help` lists `live-handoff`; `herdr server live-handoff --help` succeeds.

### fix(agent): validate restored agent names before the sanitizer can launder them

- **PR:** [cameronsjo/herdr#37](https://github.com/cameronsjo/herdr/pull/37) (issue #18)
- **Files:** `src/app/agents.rs`, `src/app/mod.rs`, `src/persist/restore.rs`, `src/terminal/state.rs`
- **Replaces:** Upstream's restore path ran restored agent names through the label sanitizer without first validating them, letting malformed persisted state through silently.
- **Regression check:** `cargo test persist::restore`.

### test: retry an unexpected docker-check failure once in isolation

- **PR:** [cameronsjo/herdr#33](https://github.com/cameronsjo/herdr/pull/33) (issue #25)
- **Files:** `justfile`, `scripts/docker-check.sh`, `scripts/test_docker_check.py`
- **Replaces:** Upstream has no `docker-check.sh` (fork-only); this fixes flaky single-run failures by isolating and retrying once before reporting a real regression.
- **Regression check:** `python3 scripts/test_docker_check.py`.

### fix(sidebar): let grouped_rows win over a per-agent override when grouped

- **PR:** [cameronsjo/herdr#32](https://github.com/cameronsjo/herdr/pull/32) (issue #23)
- **Files:** `src/config/sidebar.rs`
- **Replaces:** Upstream has no grouped-sidebar concept (fork-only feature from PR #11); this fixes a precedence bug where a per-agent override silently defeated grouping.
- **Regression check:** `cargo nextest run -E 'test(/grouped_rows/)'` (3 tests; the old `cargo test config::sidebar::grouped_rows` matched none).

### ci: give the issue-closing workflow a usable token

- **PR:** [cameronsjo/herdr#29](https://github.com/cameronsjo/herdr/pull/29)
- **Files:** `.github/workflows/label-next-release-issues.yml`
- **Replaces:** Upstream's default `GITHUB_TOKEN` can't close issues on this fork's workflow; swaps in a token scoped to do it.
- **Regression check:** `gh run list --workflow label-next-release-issues.yml -R cameronsjo/herdr --limit 1` shows a green run.

### docs: replace the stale fork notice in the README

- **PR:** [cameronsjo/herdr#28](https://github.com/cameronsjo/herdr/pull/28) (issue #19)
- **Files:** `README.md`
- **Replaces:** The fork notice from `07821baa`/`137f678f` had gone stale against the fork's actual feature set.
- **Regression check:** manual read — `README.md` top section names the fork's current divergences.

## Adaptive command palette (`docs/plans/2026-09-02-adaptive-command-palette.md`)

### feat(palette): remember recent commands and compact defaults

- **Commit:** `5a7b7874` (direct commit, no PR)
- **Files:** `src/palette_history.rs`, `src/app/{mod,state}.rs`, `src/app/input/{mod,modal,navigate,overlays}.rs`, `src/main.rs`, `docs/next/website/src/content/docs/keyboard.mdx` (en/ja/zh-cn)
- **Replaces:** Upstream's command palette always shows the full, unordered command list before the user types anything.
- **Regression check:** open the palette with no query and confirm recently-used commands surface first.

## Agent Type-Submit Primitive (`docs/plans/2026-09-01-agent-type-submit-primitive.md`)

### feat(agent): add type-submit primitive

- **PR:** [cameronsjo/herdr#27](https://github.com/cameronsjo/herdr/pull/27)
- **Files:** `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/{agent-automation,cli-reference,socket-api}.mdx` (en/ja/zh-cn)
- **Replaces:** Upstream's `agent prompt` always routes text through the model-invocable prompt path; there was no way to type literal UI text (e.g. `/compact`) and press Enter without it being interpreted as a prompt.
- **Regression check:** `herdr agent type-submit --help` succeeds; send `/compact` via type-submit to a live agent pane and confirm it appears as typed input, not a prompt.

## Sync herdr fork with upstream, then cut v0.8.2-palette.3 (`docs/plans/2026-08-26-sync-herdr-fork-with-upstream-then-cut-v0-8-2-palette-3.md`)

No section of its own beyond the `407c6e85` upstream-sync merge (listed
above) and the `v0.8.2-palette.3` tag cut. Two small fixes landed alongside
its validation pass:

### feat(sidebar): align trailing token groups

- **Commit:** `6ef60c32` (direct commit, no PR)
- **Files:** `src/config/sidebar.rs`, `src/ui/sidebar.rs`, `src/config.rs`, `docs/next/website/src/content/docs/configuration.mdx`
- **Replaces:** Upstream's sidebar (and this fork's own PR #11 grouped rows) left trailing status tokens ragged across rows within a group.
- **Regression check:** `cargo test ui::sidebar` and visually confirm trailing tokens align in a grouped workspace.

### test(git): skip unreadable ref assertion when permissions are bypassed

- **Commit:** `6e9baf36` (direct commit, no PR)
- **Files:** `src/workspace/git/discovery.rs`
- **Replaces:** A test asserting an unreadable-ref failure mode false-failed when run as a user (e.g. root, or a sandboxed CI runner) that bypasses filesystem permission checks.
- **Regression check:** `cargo test workspace::git::discovery`.
- **Retired 2026-09-17:** upstream rewrote the test to reach the same unavailable-ref branch through a symlink loop, which fails to open even as root, so the fork's skip guard has nothing left to guard. The 2026-09-17 sync resolved this file toward upstream and the divergence is gone.

## Pre-plan fork setup

Everything below predates `docs/plans/` — the fork's first ~70 commits,
before plan documents existed. Grouped here rather than dropped.

### merge: command palette and pane moves from the palette

- **Commit:** `df8d7625` (direct merge of a local branch, no PR — the founding fork commit)
- **Files:** `src/app/{actions,ids,mod}.rs`, `src/app/input/{mod,modal,navigate}.rs`, `docs/next/website/src/content/docs/keyboard.mdx` (en/ja/zh-cn), `docs/next/website/src/data/config-reference.json`
- **Replaces:** A local build of `herdrdev/herdr#2299` (command palette + pane moves), which upstream's contribution gate auto-closes as an over-budget feature. This is the change that started the fork.
- **Regression check:** open the command palette (`Ctrl+K` or configured binding) and move a pane with a palette command.

### feat(palette): add split-direction picker and plugin commands

- **PR:** [cameronsjo/herdr#5](https://github.com/cameronsjo/herdr/pull/5)
- **Files:** `src/app/actions.rs`, `src/app/api/plugins/mod.rs`, `src/app/input/{mod,modal,mouse,navigate}.rs`, `src/app/{mod,state}.rs`
- **Replaces:** Upstream splits panes in one fixed direction from the palette; this adds a direction picker and exposes plugin commands to the palette.
- **Regression check:** split a pane from the palette and confirm the direction picker appears before the split commits.

### feat(palette): match split/pane commands by cross-vocabulary keyword

- **PR:** [cameronsjo/herdr#6](https://github.com/cameronsjo/herdr/pull/6)
- **Files:** `src/ui/{keybind_help,palette}.rs`, `AGENTS.md`, `docs/next/CHANGELOG.md`
- **Replaces:** Upstream's palette search matches only a command's literal name; this adds synonym/keyword matching (e.g. "split" also matches "divide").
- **Regression check:** search the palette for a synonym of a command name (not the command's literal name) and confirm it surfaces.

### feat(palette): run tab reorder and one-keystroke pane resize from the palette

- **Commit:** `1943ed32` (direct commit, no PR)
- **Files:** `src/ui/keybind_help.rs`, `src/ui/palette.rs`
- **Replaces:** Upstream's tab-reorder and pane-resize actions arrived as plain help rows with no `NavigateAction`, so they never reached the palette despite having working handlers.
- **Regression check:** search the palette for "reorder tab" and "resize pane" and confirm both are runnable.

### feat(skill): split the agent skill into a directory the binary can emit

- **PR:** [cameronsjo/herdr#7](https://github.com/cameronsjo/herdr/pull/7)
- **Files:** `skills/herdr/SKILL.md`, `skills/herdr/references/*.md`, `src/main.rs`, `nix/package.nix`, `Cargo.toml`, `docs/next/website/src/content/docs/agent-skill.mdx`
- **Replaces:** Upstream ships (or shipped, at this fork point) the agent skill as a single flat file; this splits it into a directory the binary can emit via a subcommand.
- **Regression check:** `herdr --skill` prints the agent skill and exits (the flag is declared in `src/cli/spec.rs`).

### feat: move a tab to another space

- **PR:** [cameronsjo/herdr#8](https://github.com/cameronsjo/herdr/pull/8)
- **Files:** `src/api/schema/events.rs`, `scripts/sync-upstream.sh`, `docs/next/api/herdr-api.schema.json`, `docs/next/website/src/content/docs/{keyboard,socket-api}.mdx` (en/ja/zh-cn)
- **Replaces:** Upstream has no cross-space tab move; tabs are pinned to the space they were created in.
- **Regression check:** `herdr tab move --space <id>` (or the CLI's actual flag) relocates a tab and the socket API emits the corresponding event.

### fix: sanitize labels, require a tab move destination, cap pane aliases

- **PR:** [cameronsjo/herdr#9](https://github.com/cameronsjo/herdr/pull/9)
- **Files:** `src/label.rs`, `src/app/actions.rs`, `src/app/api/{panes,tabs,workspaces,worktrees}.rs`, `src/api/schema/tabs.rs`, `src/main.rs`
- **Replaces:** Upstream's tab-move command silently no-ops with no destination; pane aliases and workspace labels were unbounded and unsanitized.
- **Regression check:** `cargo test label::` and `herdr tab move` with no destination flag errors instead of no-opping.

### feat(sidebar): group agent rows under one header per workspace

- **PR:** [cameronsjo/herdr#11](https://github.com/cameronsjo/herdr/pull/11) (issue #10)
- **Files:** `docs/next/website/src/content/docs/{configuration,socket-api}.mdx` (en/ja/zh-cn), `docs/next/CHANGELOG.md`, `AGENTS.md`
- **Replaces:** Upstream's sidebar lists every agent row flat; this groups them under one header per workspace.
- **Regression check:** open the sidebar with 2+ agents in one workspace and confirm a single shared header.

### fix(label): sanitize cwd-derived workspace labels

- **PR:** [cameronsjo/herdr#13](https://github.com/cameronsjo/herdr/pull/13) (issue #12)
- **Files:** `src/label.rs`, `src/workspace/git/discovery.rs`
- **Replaces:** Upstream derives workspace labels from `cwd` without sanitizing path characters that break rendering or the API.
- **Regression check:** `cargo test label::sanitize_label` with a `cwd` containing control/reserved characters.

### fix(label): route the remaining label producers through sanitize_label

- **PR:** [cameronsjo/herdr#15](https://github.com/cameronsjo/herdr/pull/15) (issue #14)
- **Files:** `src/label.rs`, `src/app/actions.rs`, `src/app/api/panes.rs`, `src/app/api_helpers.rs`, `src/app/state.rs`, `src/terminal/state.rs`, `src/ui/mobile.rs`
- **Replaces:** Follow-up to `#13` — several other label producers still bypassed `sanitize_label`.
- **Regression check:** `cargo test label::sanitize_label` (full producer sweep, not just the `cwd` case from `#13`).

### fix(sidebar): stop painting the workspace header as the focused row

- **PR:** [cameronsjo/herdr#21](https://github.com/cameronsjo/herdr/pull/21)
- **Files:** `src/ui/sidebar.rs`, `docs/next/CHANGELOG.md`
- **Replaces:** A regression in this fork's own grouped-sidebar work (`#11`): the group header itself could render as the focused row instead of a real agent row.
- **Regression check:** focus the first agent in a grouped workspace and confirm the header never paints as focused.

### refactor(navigator): isolate the fork's pane-move gate into its own function

- **Commit:** `2134321c` (direct commit, no PR)
- **Files:** `src/app/actions.rs`
- **Replaces:** N/A — internal refactor to keep the fork's pane-move gate mergeable against upstream's own edits to the surrounding tab-row logic.
- **Regression check:** `cargo test app::actions::navigator`.

### chore: add docker-check and sync-upstream scripts for fork rebuilds

- **Commit:** `42ba3d3a` (direct commit, no PR)
- **Files:** `scripts/docker-check.sh`, `scripts/sync-upstream.sh`
- **Replaces:** N/A — fork-only tooling; upstream has neither script since it doesn't need a containerized fork-rebuild flow.
- **Regression check:** `bash scripts/docker-check.sh` completes and prints its `unexpected test failure(s)` line (empty on a clean tree).

### docs(agents): never touch herdrdev/herdr upstream without explicit approval

- **Commit:** `1fbf4e14` (direct commit, no PR)
- **Files:** `AGENTS.md`
- **Replaces:** N/A — codifies the fork's write-only-to-`fork`-remote rule (later restated in `CLAUDE.md`).
- **Regression check:** manual read — `AGENTS.md` states the never-write-to-`origin` rule.

### fix(sync-upstream): resume the merge after conflicts are resolved by hand

- **Commit:** `8a8bfd6f` (direct commit, no PR)
- **Files:** `scripts/sync-upstream.sh`
- **Replaces:** N/A — fixes `42ba3d3a`'s own script: its precondition guard rejected the exact resolved-conflict state it told the user to reach.
- **Regression check:** `bash scripts/sync-upstream.sh` after manually resolving a merge conflict resumes instead of aborting.

### docs(readme): describe upstream's current contribution gate

- **Commit:** `137f678f` (direct commit, no PR)
- **Files:** `README.md`
- **Replaces:** N/A — corrects the fork notice: upstream replaced its file/line-count contribution cap with an `.github/APPROVED_CONTRIBUTORS` allowlist.
- **Regression check:** manual read — `README.md` names the allowlist, not the retired file/line cap.

### docs(agents,readme,changelog): document palette split-direction/plugin commands; fix docker-check.sh staleness

- **Commit:** `b3cbf051` (direct commit, no PR)
- **Files:** `AGENTS.md`, `README.md`, `docs/next/CHANGELOG.md`, `scripts/docker-check.sh`
- **Replaces:** N/A — documents `#5`'s split-direction picker and plugin commands, and fixes `docker-check.sh` leaving `zig-out` empty on a `.rs`-only rerun.
- **Regression check:** `bash scripts/docker-check.sh` twice in a row (second run must not fail on a missing `libghostty-vt`).

### ci: install Zig from Homebrew for manual macOS builds

- **Commit:** `82fadea6` (direct commit, no PR)
- **Files:** `.github/workflows/build-artifacts-manual.yml`
- **Replaces:** N/A — CI fix: the pinned Zig 0.15.2's bundled libSystem predates the `macos-latest` runner's SDK, breaking manual-build links (mirrors what `release.yml` already did).
- **Regression check:** `gh run list --workflow build-artifacts-manual.yml -R cameronsjo/herdr --limit 1` shows a green run.

### docs: replace the README with a fork notice

- **Commit:** `07821baa` (direct commit, no PR — superseded by `#28`)
- **Files:** `README.md`
- **Replaces:** Upstream's README with a notice stating what this fork diverges on, why, and how to install it.
- **Regression check:** manual read (superseded by `#28`'s later revision).

### fix(build): realign the zig archive before linking on macOS

- **PR:** [cameronsjo/herdr#1](https://github.com/cameronsjo/herdr/pull/1)
- **Files:** `build.rs`
- **Replaces:** A macOS linker alignment bug in upstream's `build.rs` zig-archive step.
- **Regression check:** `cargo build --release` on macOS links cleanly.

### chore(ci): build macOS artifacts for Apple Silicon only

- **PR:** [cameronsjo/herdr#2](https://github.com/cameronsjo/herdr/pull/2)
- **Files:** `.github/workflows/build-artifacts-manual.yml`
- **Replaces:** Upstream's manual macOS artifact workflow also builds Intel; this fork only needs Apple Silicon.
- **Regression check:** `gh run list --workflow build-artifacts-manual.yml -R cameronsjo/herdr --limit 1` shows one macOS job (arm64), not two.

### ci: fork-local release pipeline and weekly upstream sync

- **PR:** [cameronsjo/herdr#3](https://github.com/cameronsjo/herdr/pull/3)
- **Files:** `.github/workflows/release.yml`, `.github/workflows/sync-upstream.yml`, `scripts/{conventional_commits,fork_release_notes}.py`, `scripts/{open-sync-pr,sync-upstream-ci}.sh`
- **Replaces:** N/A — fork-only CI: upstream has no fork-suffixed release pipeline or automated weekly sync-PR job.
- **Regression check:** `gh workflow list -R cameronsjo/herdr` shows `release.yml` and `sync-upstream.yml` as active.

### ci: bump the Homebrew formula from the release pipeline

- **PR:** [cameronsjo/herdr#4](https://github.com/cameronsjo/herdr/pull/4)
- **Files:** `.github/workflows/release.yml`
- **Replaces:** N/A — fork-only: bumps `cameronsjo/homebrew-tap`'s formula version as part of the fork's own release, which upstream has no equivalent of.
- **Regression check:** `gh api repos/cameronsjo/homebrew-tap/contents/Formula/herdr.rb --jq .content | base64 -d | grep version` matches the latest fork tag.

### ci: run the website workflow only on the upstream repository

- **PR:** [cameronsjo/herdr#16](https://github.com/cameronsjo/herdr/pull/16)
- **Files:** `.github/workflows/website.yml`
- **Replaces:** Upstream's website-deploy workflow would otherwise also fire on fork pushes, deploying the fork's docs site over upstream's.
- **Regression check:** `gh run list --workflow website.yml -R cameronsjo/herdr --limit 5` shows no runs triggered by fork-only pushes.

### Update maintainers list in MAINTAINERS file

- **Commit:** `01d01986` (direct commit, no PR)
- **Files:** `.github/MAINTAINERS`
- **Replaces:** N/A — updates the fork's copy of the maintainers list.
- **Regression check:** manual read — `.github/MAINTAINERS` lists the current maintainer set.
