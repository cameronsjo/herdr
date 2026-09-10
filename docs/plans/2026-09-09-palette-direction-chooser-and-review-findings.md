---
body_sha256: "18ceb29ba75276edd8c4ad2318dfc21ed1b4bfb135d89caa029587524f794af6"
session_id: "3addb1f0-9fc6-4df7-b07d-08b893d37b57"
model: "claude-fable-5-1"
harness: "claude-code 2.1.267"
machine: "cf6e768835c7"
approved_session_id: "a5fe9fd2-25da-4bf2-8bb7-355062905da9"
status: done
next: "All tasks DONE. Review findings folded in (b40bf79a). Remaining: Cameron runs the live check and merges cameronsjo/herdr#61."
branch: feat/palette-direction-picker
pr: cameronsjo/herdr#61
updated: 2026-09-09
date: 2026-09-09
---

# herdr fork: palette direction chooser, destructive confirm, and the review findings

> Persist to `herdr/docs/plans/2026-09-09-palette-direction-chooser-and-review-findings.md` (the fork keeps its plans there). Owning repo: `cameronsjo/herdr` (the `fork` remote). Never a write against `herdrdev/herdr`.

## Context

A UX review of the palette for the query `move` (screenshots 2026-09-09) found: nine `move <noun> <direction>` rows wall the list; a destructive plugin command (`Collie — Uninstall web bridge (remove service)`) surfaces on a mid-word substring hit of "remove" with no marker and no confirm; plugin rows leave the key column blank while every other row says `unset`; `swap pane` and `merge workspace` rows appear via hidden keywords with no visible reason; the `esc close` badge repeats the footer. Code confirms each: `src/client/shell/palette.rs:339-367` (rank tiers, keyword tier), `src/client/shell/overlays.rs:1076` (empty key skipped), `overlays.rs:987` (badge, which is also the palette's click-to-close rect). The bold-text artifact in the `Collie` row is a separate render bug: filed in Task 0, not fixed here.

## Goal

The `move` query shows three family rows (`move tab...`, `move workspace...`, `swap pane...`) that open a direction chooser; mid-word substring hits no longer match; destructive plugin actions are tagged and confirmed, with a fork-side config override so third-party actions can be marked; every palette row shows a key, a match reason, or a visible dash; the palette chrome says each thing once; and the fork carries a divergence ledger listing every change it holds over upstream.

## Alternatives declined

- **Second palette stage** (Enter on a family row re-filters the same list to its leaves) — cheaper by a day and no new overlay; Cameron chose the button chooser that matches the existing split-into-tab modal.
- **Collapse all directional families** (focus, resize, split too) — move + swap are the two families the review found; the rest wait for a second review.
- **Keep the substring rank tier and rely on the destructive tag** — dropping the tier is three lines and removes the bad hit at the source; the tag stays as the net for direct hits.
- **Manifest field only, or config override only** — the field is the seam a plugin author can use; the override is what makes the confirm fire on the five third-party plugins installed today. Both.
- **Skip the fork ledger** — 148 commits and 159 files ahead of upstream with only sync plans as a record is how a regression hides.

## Panel

Panel: cadence:plan-reviewer (conflict + drift), cadence:plan-reviewer (underspecified tasks), cadence:user-experience-reviewer, artificer-voice:cameron-review ran — 36 findings, 35 folded in, 1 declined (see Panel review — findings declined)

## Panel review — findings declined

- **[Task 1] Render a `terminal too narrow — widen to N columns` line when the chooser geometry returns `None`** — the palette itself returns `None` below 20 columns with no message; the chooser stacks its buttons vertically before giving up, and a second message style for one overlay is not worth the inconsistency.
- (advisory, not counted) **Rename the `[destructive]` tag to `[removes data]`** — "destructive" is a plain word and "removes data" is wrong for actions like a service restart.

## Architecture

- **Chooser overlay.** A generic `ClientShellOverlay::Chooser` replaces the two-button `PaneSplitDirection` overlay: `ClientChooserOverlay { title: String, choices: Vec<ChooserChoice>, selected: usize, return_to: Option<PaletteReturn> }`. `ChooserChoice { label: &'static str, outcome: ChooserOutcome }` with labels always fixed ASCII literals (the byte-length geometry invariant holds); any runtime text (a plugin row name) goes in the `title`, measured with `display_width`. `ChooserOutcome` has three arms: `Cancel`, `Palette { action: PaletteAction, command_id: String }` (replays a keybind or runs a plugin action, then records history), and `PaneSplit { pane_id, tab_id, target_pane_id, split: SplitDirection }` (the existing pane-move-onto-tab path). `PaletteReturn { query: String, selected: usize }` lets Esc or Cancel reopen the palette where the user left it. Geometry lays out N buttons on one row from label widths, and stacks them vertically when the row does not fit; mouse hit-test reads the rects the renderer drew (one function each, called from both, per herdr CLAUDE.md).
- **Family rows.** `PaletteAction::Chooser(DirectionFamily)` is a fourth action variant. `palette_commands` appends one synthetic row per family after the keybind rows. Leaf rows carry `family: Option<DirectionFamily>` and are hidden from a query unless one of the query's whitespace tokens is a prefix of `left`, `right`, `up`, or `down` (case-insensitive, one character is enough, so `move tab l` reaches `move tab left`). The empty-query compact list is untouched: no family or leaf row is featured.
- **Ranking.** `match_rank` loses its mid-word substring tier: exact, then name-prefix, then word-prefix; `MAX_NAME_RANK` becomes 2 and the keyword tier offsets from it as today. `command_match_rank` returns `(rank, matched_keyword: Option<&'static str>)`, and `filtered_palette_commands` returns `Vec<PaletteRow { command, matched_keyword }>` so the renderer can print the reason.
- **Destructive confirm.** `destructive: bool` (`#[serde(default)]`) rides `PluginManifestAction` only, because the palette reads `InstalledPluginInfo.actions: Vec<PluginManifestAction>` (`src/api/schema/plugins.rs:55`); `PluginActionInfo` is untouched. A fork-side config key `[palette] destructive_actions = ["<plugin_id>:<action_id>"]` marks third-party actions; a row is destructive when either source says so. Enter on a destructive row opens the chooser titled with the full row name, choices `cancel` (index 0, selected by default) and `run anyway`; history records only on `run anyway`.
- **Right column.** The renderer prints the key when the row has one, else `matched: <keyword>` when the hit was keyword-only, else a dim `—`. `KeybindHelpEntry.key` becomes `Option<String>`; the keybind help screen keeps its own `unset` fallback at its render site (`src/client/shell/render.rs:75`) and the palette never sees the word.
- **Chrome.** The `esc close` badge stays (it is the palette's cancel rect: `overlays.rs:982-991` → `OverlayRender.cancel` → `mouse.rs:1532,1620,1682,1786`). The footer becomes `" run enter · move ↑↓"`, action-first like the other five footers.

## Tech Stack

Rust (herdr fork, `master` at `24720e97`), ratatui client shell, `just check` (fmt + nextest + maintenance tests), Zig 0.15.2 for libghostty-vt. Build natively with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (see `~/.claude/CLAUDE.md` § herdr).

## Global Constraints

- All writes target the `fork` remote (`cameronsjo/herdr`). No issue, PR, or push to `origin` (`herdrdev/herdr`).
- The only new wire-visible field is `PluginManifestAction.destructive`, optional JSON with a `false` default. `tests/fixtures/endpoint-method-shapes-v1.json` is never edited: its digest hashes request schemas only (`client_commands.rs:252-282`), and this is a response type. `docs/next/api/herdr-api.schema.json` is regenerated with `HERDR_UPDATE_API_SCHEMA=1` when `generated_protocol_schema_artifact_is_current` (`src/api/schema/tests.rs:196-222`) goes red.
- Renderer and mouse hit-test share one rect function per overlay; no rect computed twice.
- No `unwrap()` in production code; `tracing` for logging; no new dependencies.
- Chooser button labels are fixed ASCII literals; runtime text goes in the title.
- Every test that names a collapsed row or the split picker is updated in the task that changes the behavior, never deleted without a replacement. `clicking_the_rendered_esc_close_button_closes_the_palette` (`tests/palette.rs:196`) stays.
- Commit style: lowercase conventional commits, no closing keywords; producer-tuple trailers omitted (fork of a third party's project, git-workflow § Commit Provenance).
- Changelog: herdr forbids per-feature edits to `docs/next/CHANGELOG.md`; the fork ledger entry (Task 6) is this change's changelog entry.
- Reports dir: the orchestrator runs `mktemp -d /tmp/herdr-palette.XXXXXX` at dispatch time, `Write`s an empty stub per task, and substitutes the absolute path for `<reports-dir>` in each dispatched task block.

## Orchestrator

**Driver:** opus — trigger: Task 1 refactors an overlay whose state, geometry, mouse routing, and tests are coupled, and Task 3 is a confirm gate on a destructive action (security judgment does not economize, U9). Tasks 4 and 5 go to fresh Sonnet implementers from their specs.

---

## Tasks

### Task 0 — Worktree, branch, draft PR, bold-artifact issue

**Files:** none in-tree.

**Dispatch:** In-context. **Report:** —

**Steps:**
- [x] `git -C ~/Projects/cadence-ecosystem/herdr worktree add ../herdr-worktrees/palette-direction-picker -b feat/palette-direction-picker master`
- [x] Copy this plan to `docs/plans/2026-09-09-palette-direction-chooser-and-review-findings.md` in the worktree; commit `docs: plan the palette direction chooser and review fixes`
- [x] `git push -u fork feat/palette-direction-picker`; `gh pr create -R cameronsjo/herdr --draft` with `--body-file`
- [x] File the bold-artifact issue on `cameronsjo/herdr` with `--body-file`: title `palette: stray bold spans inside plugin row labels`; body: two screenshots at different terminal sizes show bold on the same substring (`all web`) of the `Collie` row, which rules out bleed-through from the pane beneath; `overlays.rs:1060-1075` paints one style per row, so the modifier arrives with the row text or survives a frame diff. Reproduction: open the palette, type `move`, look at the `Collie` row.

### Task 1 — Generic chooser overlay; split picker migrated onto it [DONE]

**Files:**
- Modify: `src/client/shell/state.rs:183-184,378,498,744` (replace `ClientPaneSplitOverlay` + `ClientShellOverlay::PaneSplitDirection` with `ClientChooserOverlay` + `ClientShellOverlay::Chooser`; hit rects `pane_split_vertical`/`pane_split_horizontal` become `chooser_buttons: Vec<Rect>`)
- Modify: `src/client/shell/overlay_input.rs:249-330,432` (`open_chooser_overlay(title, choices, return_to)`; `route_chooser_key`: Esc → `Cancel`; Left/Right/Up/Down/Tab/BackTab move `selected` with wrap; Enter runs the selected outcome; `v`/`h` shortcuts only when the choices are the two split outcomes. `Cancel` with `return_to: Some` reopens the palette with that query and selection, else closes. `open_pane_split_direction_overlay` builds a two-choice chooser titled `split into tab` with `PaneSplit` outcomes)
- Modify: `src/client/shell/overlays.rs:1110+` (`render_chooser_overlay`: title, N buttons, selected button in accent, vertical stack when the row does not fit), `src/client/shell/palette.rs:25-26,31-115` (`chooser_geometry(area, labels) -> Option<(Rect, Rect, Vec<Rect>)>`; `split_button_labels` stays as the two split literals)
- Modify: `src/client/shell/composition.rs:599`, `src/client/shell/mouse.rs:1360,1816-1826` (hit-test iterates `chooser_buttons`; every other event past the chooser is swallowed), `src/client/shell/input_source.rs:11`
- Test: `src/client/shell/tests/palette.rs:263,337,375` (renamed to the chooser), `src/client/shell/tests/mouse_selection.rs:1018`; new `a_four_way_chooser_cycles_and_runs_the_selected_outcome`, `cancel_from_a_palette_spawned_chooser_restores_the_query`, `a_chooser_too_wide_for_one_row_stacks_its_buttons`

**Interfaces:**
- Produces: `ChooserChoice`, `ChooserOutcome::{Cancel, Palette{action, command_id}, PaneSplit{..}}`, `PaletteReturn`, `open_chooser_overlay`.

**Dispatch:** Serial (wave 1) · in-context Opus. **Report:** —

**Steps:**
- [x] Write the three new tests; expect RED
- [x] Introduce the chooser types and geometry; migrate the split picker; update the four existing tests
- [x] `just test`; expect GREEN
- [x] Commit: `refactor(client): generalize the split picker into a chooser overlay` (de500b47)

### Task 2 — Family rows, leaf filter, ranking, match reason [DONE — c94ee161]

**Files:**
- Modify: `src/client/shell/palette.rs` (`PaletteAction::Chooser(DirectionFamily)`; `DirectionFamily::{MoveTab, MoveWorkspace, SwapPane}` with `fn choices(&self) -> Vec<ChooserChoice>` mapping `left`/`right` → `MoveTabPrevious`/`MoveTabNext`, `up`/`down` → `MoveWorkspacePrevious`/`MoveWorkspaceNext`, `left`/`down`/`up`/`right` → `SwapPane*`, each `Palette` outcome carrying the leaf row's `command_id`; `PaletteCommand.family: Option<DirectionFamily>`; `palette_commands` appends `move tab...` (keywords `["reorder tab", "tab left", "tab right"]`), `move workspace...` (`["reorder workspace", "workspace up", "workspace down"]`), `swap pane...` (`["move pane", "reorder pane"]`); `match_rank` drops the `contains` tier, `MAX_NAME_RANK = 2`; `command_match_rank` returns the matched keyword; `filtered_palette_commands` returns `Vec<PaletteRow>` and hides leaf rows unless a query token prefixes a direction word)
- Modify: `src/client/shell/palette/input.rs:113-150` (callers read `row.command`; `run_palette_action` arm for `Chooser`: `open_chooser_overlay(family name without the ellipsis, family.choices(), Some(PaletteReturn{query, selected}))`)
- Modify: `src/input/keybind_help.rs:237-296,418-441` (leaf entries gain `family:`; the `move pane <dir>` synonyms move off the swap rows onto the family row)
- Test: `src/client/shell/palette.rs` — `reorder_and_resize_rows_reach_the_palette` asserts the three family rows plus resize leaves; `move_pane_left_matches_swap_pane_left_via_keyword` becomes `move_pane_reaches_the_swap_family_row`; `reorder_workspace_matches_both_workspace_move_commands_via_keywords` asserts `reorder workspace` ranks `move workspace...` first; `a_word_prefix_outranks_a_mid_word_substring` becomes `a_mid_word_substring_does_not_match`; new `a_move_query_shows_one_row_per_family`, `a_query_naming_a_direction_prefix_reaches_the_leaf_row` (`move tab l` → `move tab left` first), `enter_on_a_family_row_opens_the_chooser_with_a_return`, `a_keyword_only_match_reports_its_keyword`

**Interfaces:**
- Consumes: `open_chooser_overlay`, `ChooserOutcome` (Task 1). Produces: `DirectionFamily`, `PaletteRow`, `PaletteCommand.family`.

**Dispatch:** Serial (after Task 1) · in-context Opus. **Report:** —

**Steps:**
- [x] Tests first; expect RED
- [x] Implement; keybind chords unchanged; `EMPTY_PALETTE_LIMIT` and `FEATURED_COMMAND_IDS` untouched; the `...` suffix means "asks you something next", the same reading as `merge workspace into...`
- [x] `just test`; expect GREEN
- [x] Commit: `feat(client): collapse move and swap rows into a direction chooser`

### Task 3 — Destructive plugin actions: manifest field, config override, tag, confirm [DONE — b4390d7f]

**Files:**
- Modify: `src/api/schema/plugins.rs:244` (`destructive: bool`, `#[serde(default)]`, on `PluginManifestAction` only) and the manifest parser that fills it; `docs/next/api/herdr-api.schema.json` (regenerate, see Global Constraints); `docs/next/website/src/content/docs/plugins.mdx` (manifest field) and `configuration.mdx` + `src/data/config-reference.json` (the config key)
- Modify: the client config struct that carries `LiveKeybindConfig` (locate via its constructor) — add `palette.destructive_actions: Vec<String>`, reloadable with `herdr server reload-config` like the rest of the config
- Modify: `src/client/shell/palette.rs:108,249-296` (`PaletteCommand.destructive`; true when the manifest says so or `"{plugin_id}:{action_id}"` is in the override list; label suffix ` [destructive]`), `src/client/shell/palette/input.rs:113-150` (`run_palette_selection`: a destructive row opens the chooser titled with the full row name, choices `cancel` / `run anyway`, `selected = 0`, `return_to = Some(..)`; `remember_palette_command` runs inside the `Palette` outcome, never before), `src/client/shell/overlays.rs` (tag in the palette's warning color; the selected-row style wins)
- Test: `src/client/shell/palette.rs` — fixture: extend `test_plugin()` / `host_plugins()` (`palette.rs:893-964`) with one destructive action; `a_destructive_manifest_action_carries_its_tag`, `a_config_override_marks_a_third_party_action_destructive`, `an_old_manifest_without_the_field_parses_as_not_destructive`, `enter_on_a_destructive_row_confirms_before_running`, `a_destructive_confirm_defaults_to_cancel`, `a_cancelled_destructive_row_is_not_remembered`

**Interfaces:**
- Consumes: chooser (Task 1), `PaletteRow` (Task 2). Produces: manifest field `destructive`, config key `palette.destructive_actions`.

**Dispatch:** Serial (after Task 2) · in-context Opus. **Report:** —

**Steps:**
- [x] Tests first; expect RED
- [x] Implement; `just test`; regenerated `docs/next/api/herdr-api.schema.json` (one additive optional boolean)
- [x] Commit: `feat(plugins): tag and confirm destructive palette actions`

### Task 4 — Right column and footer [DONE — 17fa1c49]

**Files:**
- Modify: `src/input/keybind_help.rs:14,77,82` (`KeybindHelpEntry.key: Option<String>`; the builders stop substituting `unset`), `src/client/shell/render.rs:75` and `src/config/keybinds.rs:836` (each keeps its own `unset` fallback at the render site), `src/client/shell/palette.rs:305-332` (`PaletteCommand.key: Option<String>`)
- Modify: `src/client/shell/overlays.rs:1076-1079,1088-1093` (right column: key, else `matched: <keyword>` in `p.overlay0`, else `—` in `p.overlay0`; footer `" run enter · move ↑↓"`), and the string assertion at `overlays.rs:1272`
- Test: `src/client/shell/tests/palette.rs` — new `an_unbound_row_shows_a_dash_not_a_hole`, `a_keyword_only_row_shows_its_match_reason`, `a_plugin_row_with_no_key_and_no_keyword_shows_a_dash`; `clicking_the_rendered_esc_close_button_closes_the_palette` unchanged and green

**Interfaces:**
- Consumes: `PaletteRow` (Task 2), `PaletteCommand.destructive` (Task 3).

**Dispatch:** Serial (after Task 3) · fresh Sonnet subagent. **Report:** `<reports-dir>/task-4.md`

**Steps:**
- [x] Tests first; expect RED
- [x] Implement; `just test`; expect GREEN
- [x] Commit: `fix(client): palette right column shows a key, a match reason, or a dash`

### Task 5 — Fork divergence ledger [DONE — branch docs/fork-ledger, 9f4c31de]

**Files:**
- Create: `docs/fork/CHANGES.md` — one section per fork change, newest first: title, the fork PR (`cameronsjo/herdr#N`) or the commit SHA when the change landed without one, files or subsystems touched, the upstream behavior it replaces, and a one-line regression check (how to see it still works after a sync). Seeded from all first-parent commits in `git log --first-parent origin/master..HEAD` (58 on 2026-09-09: 39 merges, 19 direct), grouped by the seven sync plans in `docs/plans/`. Upstream-sync merges get one line each, not a section.
- Modify: `CLAUDE.md` § Fork operating rule (fork-only text, so an upstream sync cannot touch it) — one sentence: every fork PR adds its entry to `docs/fork/CHANGES.md`, and each upstream sync re-runs the regression checks listed there.

**Dispatch:** Parallel with Tasks 1-4 (disjoint files) · fresh Sonnet subagent. **Report:** `<reports-dir>/task-5.md`

**Steps:**
- [x] Generate the first-parent list; write the ledger; every PR number cited resolves with `gh pr view -R cameronsjo/herdr`, every SHA with `git cat-file -t`
- [x] Commit: `docs(fork): add the divergence ledger` (fccfcbc8)

### Task 6 — Ledger entry, polish, ship [DONE except the live check]

**Dispatch:** In-context. **Report:** —

**Steps:**
- [x] Add this PR's entry at the top of `docs/fork/CHANGES.md` (subsystems: palette, chooser overlay, plugin manifest, config; regression check: type `move`, expect three family rows and a tagged plugin row)
- [x] `just check` in the worktree — fmt and clippy clean; tests fail only on the two `live_handoff` tests that fail on `master` too
- [x] Ran `cadence-forge:polish`; its built-in arms diff the session cwd, so run `cadence:code-reviewer` and `cadence-forge:security-reviewer` on `git diff master...HEAD` built from the worktree; `cadence-forge:polish docs` for the ledger and `CLAUDE.md`; fold findings
- [ ] Manual check in a live herdr: `env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- ...` per herdr CLAUDE.md; type `move`; expect three family rows, no `Collie` row; type `uninstall`; expect the tagged row and the confirm on Enter once `palette.destructive_actions` names it. Cameron screenshots it.
- [ ] Flip the PR ready on `cameronsjo/herdr`; tick this plan; `status: done`. Merge is Cameron's.

---

## Verification

- `just test` green in the worktree, every new test name present in the nextest output.
- Screenshot of the `move` query in a running fork build matches the Goal line.
- `gh pr view -R cameronsjo/herdr <n> --json isDraft,statusCheckRollup` reads ready and green; `docs/fork/CHANGES.md` top entry names that PR.

## Deviations

- **Task 3:** the destructive-tag colour is `palette.peach` (the theme's documented warning colour). The plan said "the palette's warning color"; no `warning` field exists on the colour palette.
- **Task 3:** the override list rides `PalettePlugins` rather than a new `filtered_palette_commands` parameter, matching how `recent_command_ids` is already carried on the palette overlay. Same effect, no new parameter through two render paths.
- **Task 3:** added `a_destructive_override_matches_the_whole_pair_not_either_half` and `run_anyway_runs_the_destructive_action_and_remembers_it` beyond the plan's list — the first pins that marking one action never marks a sibling, the second is the positive arm the plan only specified negatively.

- **Task 2:** `match_rank` drops the mid-word tier for a WORD-BOUNDARY substring tier rather than removing substring matching outright. The plan's three-tier scheme broke `new tab` finding `move pane to new tab` (a multi-word query can never be a single-word prefix). The boundary is any non-alphanumeric character, so `remove` still finds `(remove service)` while `move` inside `remove` does not — the review's actual finding.
- **Task 2:** the `move pane <dir>` synonyms STAY on the swap leaf rows instead of moving to the family row. Moving them left `move pane left` matching nothing at all, because the leaf rows only surface on a directional query and the family row's own keywords do not carry a direction. `src/input/keybind_help.rs` is unchanged by this task.
- **Task 2:** `DirectionFamily` is derived from the leaf `KeybindAction` rather than declared as a `family:` field on `KeybindHelpEntry`. One source, so a family and its leaves cannot drift, and `src/input` keeps no dependency on a client-shell type.
- **Task 2:** a family row is NOT recorded in palette history — it asks rather than runs, so recording it would head the empty palette with a question and double-record the leaf the chooser then runs.

- **Task 1:** `chooser_geometry` measures labels with `display_width` rather than byte length. The plan leaned on the ASCII byte-length invariant; measuring directly makes the rect correct by construction, and the ASCII test survives retargeted as `chooser_labels_are_ascii_so_every_terminal_renders_them_the_measured_width`.
- **Task 1:** `ChooserOutcome::Cancel` is deferred to Task 3, which is where its first constructor (the `cancel` button on a destructive confirm) lands. Shipping it in Task 1 would have meant a dead variant and a `dead_code` warning across two commits.
- **Task 1:** three tests fail in this worktree for reasons predating the branch — `live_handoff_preserves_pane_process_io` and `live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session` fail identically on `master` (verified in the primary checkout at 24720e97); `client_read_loop_rejects_oversized_bracketed_paste_without_disconnect` and `server_reload_agent_manifests_reports_runtime_override` are load-flaky, each failing once under the full parallel run and passing on three isolated re-runs.


### Task 6 — review findings folded in

Two reviewers ran against `git diff master...HEAD` (the repo's own polish preflight resolves the base to upstream `origin/master` and returns the whole 165-file fork divergence, so it was not usable here).

- **cadence:code-reviewer** — 0 Critical, 1 Important, 2 Nits. The Important was real: `run_chooser_choice` took the overlay and then returned early on an out-of-range index without restoring the palette, losing the operator's query. Fixed with a `tracing::warn!` and the same restore cancelling does, pinned by `an_out_of_range_chooser_index_restores_the_palette_rather_than_closing_it`.
- **cadence-forge:security-reviewer** (opus, U9) — 0 Critical, 2 Important, 4 Nits.
  - **Fixed:** `action_is_marked_destructive` split the override entry on the first colon, but the manifest's identifier rules allow a colon in BOTH ids. A namespaced plugin id was therefore impossible to mark — the fail-open direction, in the one lever an operator has against a plugin that declines to mark itself. Now compares the joined string; the remaining collision over-marks, which only ever costs an extra confirm. Pinned by `a_colon_in_either_id_is_markable_and_a_collision_errs_toward_confirming`.
  - **Fixed:** the confirm dialog truncated the plugin-chosen name it was asking about, so a plugin could front-load innocuous text. `chooser_geometry` now takes the title and widens the popup to fit it.
  - **Documented, not built:** `destructive` binds the palette only — a link handler or `herdr plugin action invoke` still runs the action with no confirm. Routing those through a confirm is a new flow on a different surface and outside this plan's goal; `plugins.mdx` now says the key is not an authorization boundary.
  - **Declined:** a whole-file config parse error drops the override list (the live-reload path already guards it); a long plugin name can push the `[destructive]` tag off the row (cosmetic, the Enter gate still fires); the restored palette selection is briefly stale until the plugin list arrives (self-correcting).

## Learnings

- **`cargo check --tests` is not the CI gate, and it masks a whole class of failure.** CI runs `just lint` = `cargo clippy --all-targets --locked -- -D warnings`. A field read only by a `#[cfg(test)]` assertion compiles clean under `cargo check --tests` and fails CI with `error: field is never read` on the non-test build — exactly what `PaletteRow.matched_keyword` did between Task 2 and Task 4, turning the branch's CI red for two commits. Run clippy with warnings denied before calling a branch green.
- **`just` is not on PATH in the Bash tool.** It lives at `~/.local/share/mise/installs/just/1.58.0/just`, and the mise shim refuses without a global default version pinned. Invoke the binary by absolute path, or run the underlying cargo commands directly.
- **Two `live_handoff` tests fail on `master`** — `live_handoff_preserves_pane_process_io` and `live_handoff_keeps_unmanaged_agent_name_bound_to_saved_session`, verified in the primary checkout at `24720e97`. Three more are load-flaky under the full parallel run and pass in isolation. Neither set is a signal about a branch.
- **Dropping a match tier is not the same as dropping mid-word matching.** The plan's three-tier scheme silently broke `new tab` finding `move pane to new tab`, because a multi-word query can never be a single-word prefix. A word-boundary substring tier kills the bad hit the review found and keeps every good one; the test that caught it was an existing one, not a new one.

