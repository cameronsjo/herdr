---
status: "in-flight"
updated: "2026-09-04"
branch: "master"
body_sha256: "ed963b62c5deeacd53ff2146f6b89c53956eb4d0652295f411f5b5692087c4c8"
session: "woven-reed"
session_id: "38b44176-e85a-4ede-9097-197a709ce939"
machine: "cf6e768835c7"
approved_in: "keen-chisel"
approved_session_id: "68cf1987-af5a-4173-8a26-03b822f8065e"
---

# herdr fork: land the queue, sync upstream, close the issues, fix reshuffle

## Context

The `cameronsjo/herdr` fork has three backlogs that have drifted apart, plus one capability gap
Cameron hit in daily use.

**Two PRs are parked green.** `#26` (Codex lifecycle and pane titles) and `#27` (agent
type-submit primitive) are both `MERGEABLE` / `CLEAN` with every CI check passing and **no human
review** — `#27` has a CodeRabbit comment only. They have sat since 2026-08-28 and 2026-09-03.

**The upstream sync is overdue and harder than the last one.** `fork/master` is 37 behind and 76
ahead of `origin/master`. 47 files overlap between the two diffs, against 3 conflicts in the
2026-08-26 sync. Upstream landed `refactor: render the shell in the client` (`207be3c7`), which
restructured `src/protocol/wire.rs` — the handshake moved from line ~1010 to ~1701.

**Issue `#17`'s predicted collision already happened, silently.** The fork bumped
`PROTOCOL_VERSION` `20 → 21` in `67d36eaa`; upstream independently bumped `20 → 21` in
`d79fd746` (`fix: require explicit workspace group close`, `#3206`). The 2026-08-26 sync merged
them with **no conflict** — both said `21` — so the fork and upstream now ship different wire
formats under one number, and the handshake check that exists to catch exactly this cannot see
it. Upstream has since moved to `22` (in `99c23cd1`, not the shell refactor). Merging again
converges the number and reproduces the collision.

**The palette work Cameron asked about is already built but unreleased.** `feat(palette):
remember recent commands and compact defaults` (`5a7b7874`) is `fork/master`'s HEAD. The latest
tag `v0.8.2-palette.3` (`407c6e85`) is an ancestor of it but predates it, and Cameron's binary
reports `0.8.2-palette.3`. Nothing needs building — it needs a tag.

**Reshuffle is mostly built; the by-name picker already exists.** `MoveTabToSpace` /
`MovePaneToSpace` arm the navigator, which has a live search field — moving a tab or pane to a
space in the TUI never required knowing an id. Only the CLI does. The real gaps are workspace
reorder (mouse-drag only), the four palette-invisible `SwapPane*` actions, five move actions with
no bindable config field, no pane drag-move, tab drag confined to its own workspace, and no
workspace merge at all.

Intended outcome: an empty PR queue, a fork rebuilt on current upstream with a protocol version
that cannot collide again, five issues closed, every move/reorder cell reachable from keyboard,
palette, CLI, mouse, and context menu, and two releases.

## Orchestrator

**Driver:** opus

Guard, protocol, and input-validation logic in scope, plus a 47-file conflict surface where a
clean merge is not a safe merge. Implementation lanes for the mechanical issues (`#23`, `#24`,
`#25`, `#19`) may run at Sonnet; step 0, `#17`, `#18`, the sync, and `workspace.merge` stay Opus.

Execution runs through `cadence-forge:run-the-gambit` — one worktree per unit under
`../herdr-worktrees/<slug>`, PRs to `fork` (`cameronsjo/herdr`) only.

## Constraints

- All branches, commits, and PRs target `fork`. `origin` is upstream and is **read-only**.
  *(Panel note: the leak risk is lower than assumed — `.git/config` sets `push.followTags = false`,
  overriding the global `true`, and `branch.master.remote = fork`. Keep `--no-follow-tags` anyway.)*
- **Diff every PR against `fork/master`, never `origin/master`.** `origin` is upstream, so
  `git diff origin/master...HEAD` resolves its base to the last *sync point* and carries all 76
  already-merged fork commits: 94 files instead of 21 for `#27`. This governs both reviewer
  dispatches and every verification recipe.
- `just check` cannot run on this Mac (Zig 0.15.2 vs the macOS 26 SDK). `scripts/docker-check.sh`
  is the local substitute (~12 min), and fork PR CI runs `check` on macOS, Ubuntu, and Windows.
  **`docker-check.sh` derives its root from `${BASH_SOURCE[0]}`** — run it by absolute path from
  the tree under test, or it tests a different tree and prints `PASS`.
- Feature work starts in a worktree; the primary checkout is protected.
- Normal work must not edit `docs/next/CHANGELOG.md` (repo CLAUDE.md:214). `#19` touches root
  `README.md` under the same file's "focused correction" exception, stated in the PR body.
- **`refs #N` closes issues at merge, not at release.**
  `.github/workflows/label-next-release-issues.yml` fires on *push to `master`*. Commits use
  lowercase conventional style, no emoji, no AI co-author lines, `refs #<n>` in the body, and no
  closing keywords.
- `fork/master` is **unprotected** (`protected: False`), so nothing mechanically stops a direct
  push. Every unit goes through a PR regardless.
- A binary change needs a **server restart**, which kills every live pane. No restart without
  Cameron's say-so.

## Workstreams

### 0 — Security review of the control file set *(new; before steps 3.3, 4.1, 5.8)*

The panel established that **no merged fork PR has ever had a human review** — `gh pr list -R
cameronsjo/herdr --state merged --json reviews` returns `[]` on every one, including `#13` and
`#15`, the two PRs that built the `sanitize_label` control this plan leans on. The diff-scoped
reviews at steps 1.2 and 3.5 see the *change*, not the never-audited mechanism it activates.

Dispatch `cadence-forge:security-reviewer` (Opus) over: `src/app/agents.rs`, `src/label.rs`,
`src/terminal/state.rs`, `src/persist/restore.rs`, `src/app/terminal_targets.rs`,
`src/protocol/wire.rs`, `src/cli/protocol_guard.rs`, `src/server/autodetect.rs`,
`src/server/handoff.rs`, `src/update.rs`, `src/api/server.rs`, `src/server/socket_paths.rs`,
`src/app/api/workspaces.rs`. Findings feed steps 3.3, 4.1, and 5.8 before they are implemented.

### 1 — Land the parked PRs

1.1 Dispatch `cadence:code-reviewer` against `git diff fork/master...HEAD` for **`#26`**
    (`feat/codex-integration`, 12 files). Post the marked review on the PR.
1.2 Same for **`#27`** (`feature/agent-type-submit`, 21 files against the correct base). It
    touches the JSON API surface — seat `cadence-forge:security-reviewer` (Opus) alongside.
1.3 Merge `#26` first, then re-check `#27`. **`BEHIND` is acceptable** (merges after a CI re-run);
    only `DIRTY` blocks. After both land, remove the two now-dead worktrees
    (`../herdr-worktrees/agent-type-submit`, `.local/worktrees/codex-integration`). Do not read
    `git branch -d` as a merge proof — both track `fork`, so it succeeds with only a `warning:`.

### 2 — Release `v0.8.2-palette.4`

2.1 **Gate on `master` CI going green after step 1's merges** — `release.yml` runs no tests, so
    the tag is the last unguarded step. Confirm the tip with `git ls-remote fork master`, not a
    cached SHA, and confirm the tag matches `Cargo.toml`'s version: the fork's pipeline **dropped**
    upstream's tag-vs-`Cargo.toml` check, and `release.yml` fires on any `v*` tag matching only a
    shape regex (`scripts/fork_release_notes.py:21`). A well-shaped wrong tag builds, releases,
    and dispatches a tap bump onto real machines with no human gate.
2.2 **Before pushing, confirm the M5 fleet machine does not auto-upgrade.** The tap bump reaches
    it if `brew upgrade` or `herdr update` runs on a timer there — which would change the binary
    under ~10 live Claude panes. One `!`-prefix check on the M5 settles it.
2.3 Push the tag with `--no-follow-tags`. Watch `Release` to green: build, release, tap-bump.
2.4 Confirm `brew upgrade herdr` lands `0.8.2-palette.4`. `herdr --version` reports a bare `0.8.2`
    by design — the suffix lives only in the tag and the tap formula.

### 3 — Upstream sync, with `#17` settled inside it

3.1 **Check `.github/workflows/sync-upstream.yml` first** — a scheduled sync runs Mondays 08:23
    UTC and opens its own date-named branch, which collides with a hand-run
    `scripts/sync-upstream.sh` (same `sync-upstream-$(date +%Y%m%d)` naming). Disable or confirm
    it will not fire before starting.
    **The script's printed "Next steps" 2–4 tell the operator to push straight to `master`
    (`sync-upstream.sh:100`). Those are superseded by 3.5.** That echo is what the operator reads
    at the exact moment the merge lands, and `master` is unprotected.
3.2 Resolve conflicts by hand. **A clean merge is not a safe merge here** — fork-widened
    signatures merge without conflict and fail only at clippy, and `docs/next/CHANGELOG.md`
    resolved `--theirs` after an upstream release cut silently drops fork entries. Read every
    auto-merged hunk in `src/app/input/`, `src/ui/sidebar.rs` **and** `src/ui/sidebar/tokens.rs`
    (a directory-scoped pass misses the single-file module), and `src/app/api/`.
    **On a resumed run the script does `git checkout --ours README.md` unconditionally**
    (`:73-76`) whenever `README.md` is still `UU` — so a partial hand-merge of that file is
    discarded, with an echo that reads as helpful automation. Stage `README.md` before re-running.
3.3 **Settle `#17`.** Take upstream's number as the base and encode the fork's divergence as
    `1000 + <upstream>` (upstream `22` → fork `1022`). **Run `grep -rni 'protocol.*\b(21|22)\b'`
    as an inventory *before* editing** — case-insensitively, because the most important site is
    `tests/support/mod.rs:18 pub const CURRENT_PROTOCOL: u32 = 21;` and a case-sensitive grep
    cannot match it. The nine sites: `tests/cli/sessions.rs:392,425,437,447,461,466`,
    `tests/api_ping.rs:307`, `tests/support/mod.rs:18`, `docs/next/api/herdr-api.schema.json:3`.
    **Drop `scripts/changelog.py:135` from the list** — it parses the constant by regex and
    carries no literal.
    **`website/latest.json` and `website/preview.json` are not inert.** Their `protocol` field is
    read as `ReleaseInfo.target_protocol` (`src/update.rs:417`, `:504`), becoming the live-handoff
    `expected_protocol` (`src/update.rs:1588` → `src/server/handoff.rs:248`, which rejects on
    inequality) and the restart-needed decision (`src/update.rs:828-829`). Decide deliberately:
    either prove the fork's `herdr update` path is unreachable under brew-only distribution and
    say so, or move the manifest number with the offset.
    **Fix the handshake diagnostic in all three places** — `src/protocol/wire.rs:1010-1018`,
    `src/cli/protocol_guard.rs:21`, `src/server/autodetect.rs:158,170`. Today they branch only on
    `<` / `>`, so an upstream client against a fork server reads *"please upgrade your herdr
    client"* — a dead end that is how a correct refusal gets worked around. Add an arm for exactly
    one side being `>= 1000` that names the two distributions instead.
    Add a comment at `src/protocol/wire.rs` stating the scheme, **and the rule for a fork-side
    wire change** (step 5.8 adds one): the fork's number is `1000 + upstream` plus a fork
    increment, so a fork bump does not contradict the offset or make the next sync's
    recomputation ambiguous.
    Note in that comment: after this lands, a `palette.4` client (`21`) cannot attach to a
    `palette.5` server (`1022`). `live-handoff` still works — `src/server/handoff.rs:247-250`
    enforces `expected_protocol` only when the flag is supplied.
3.4 Validate: `bash <absolute-path-to-sync-worktree>/scripts/docker-check.sh > /tmp/dc.log 2>&1;
    echo $?`. **Absolute path, from the worktree under test** — run from the primary checkout the
    script tests the pre-merge tree and prints `PASS`. Assert `/tmp/dc.log` names the merge commit.
3.5 Open the sync PR to `fork`, review at head (`cadence:code-reviewer` +
    `cadence-forge:security-reviewer` — the protocol change is a compatibility control), merge.
    **Before merging, scan the 37 upstream commit bodies for `^\s*refs #([0-9]+)\s*$`** — the
    close-on-push workflow will run upstream's refs against the *fork's* issue numbers. Upstream
    refs are ~3000 and fork issues ≤27, so a collision is unlikely but unbounded, and this is the
    one event that would fire it.

### 4 — The issue slate

4.1 **`#18` — restore path bypasses `valid_agent_name`.** *Opus lane.* Two panel seats disagreed
    and both constraints hold, so the shape is fixed:
    - **Validate in the setter, on the raw input, before `sanitize_label` runs.**
      `src/label.rs:7-10` documents the setter-side pattern deliberately — per-call-site
      sanitizing is what left restore unfiltered in the first place — so a check bolted into
      `restore.rs` reintroduces the defect that comment exists to prevent. But post-sanitize
      validation is **strictly worse than none**: `src/label.rs:71` strips `U+200B`, so
      `rev<ZWSP>iewer` sanitizes to `reviewer` and *passes*, converting a name that would have
      been rejected into one indistinguishable from the user's real pane.
    - `valid_agent_name` is private (`src/app/agents.rs:15`) — widen to `pub(crate)`.
    - **On the reject path call `restore_managed_agent(String::new(), agent)`, not an early
      `continue`.** That one call sets the name *and* the `ManagedAgent` record
      (`src/terminal/state.rs:2034`); skipping it silently produces a non-managed pane, changing
      resume, respawn, and detection authority. `set_agent_name("")` yields `agent_name = None`
      cleanly — the comment at `:2035-2037` anticipates it. The bare `set_agent_name` at
      `restore.rs:648` *can* just be skipped.
    - **Add restore-time dedupe**, keep-first plus log-and-skip. `valid_agent_name` says nothing
      about duplicates, and `agent_name_conflicts` (`src/app/agents.rs:399`) is called only from
      the rename and start sinks. Two panes literally named `reviewer` both pass validation, and
      the resulting `Ambiguous` error is a denial reachable by anyone who can write the session
      file.
    - **Ship a migration note.** This drops every already-stored name that is uppercase,
      non-ASCII, or `>32` **bytes** (`valid_agent_name` bounds bytes, not chars, so a short
      multibyte name fails). Log once at restore rather than renaming panes silently.
    - Correction to the issue body: a 200-char name is not reachable — `sanitize_label` caps at
      `MAX_LABEL_CHARS = 128` (`src/label.rs:18,42`) before anything is stored.
    - **Verification must be able to go red.** The ambiguity assertion in the original plan tested
      *existing* behavior and would pass against unmodified code. Test instead: the raw ZWSP name
      is rejected at the setter boundary, the rejected pane still reports `managed_agent` active
      with no name, and the dedupe drops a second identical name.
4.2 **`#23` — `rows_by_agent` beats `grouped_rows`.** `rows_for_agent`
    (`src/config/sidebar.rs:412-424`) never checks the `grouped` flag, so an override written for
    the ungrouped layout prints the space name twice under a grouped header. Take the fallback
    shape (defer to `grouped_rows` for suppressed tokens) over a parallel `grouped_rows_by_agent`
    table. One `if` plus a test. Sole production caller: `src/ui/sidebar/tokens.rs:45`.
4.3 **`#24` — `live-handoff` missing from `herdr server --help`.** The clap spec
    `server_command()` (`src/cli/spec.rs:168-187`) omits it entirely while the dispatch arm
    (`src/cli/server.rs:10`) and hand-written help (`:259`) both have it. Add the `.subcommand(...)`
    with `--import-exe`, `--expected-protocol`, `--expected-version` (matching the usage string at
    `src/cli/server.rs:199`). Also fixes completions and satisfies `src/cli/spec.rs:1079`.
4.4 **`#25` — flaky `agent_start_stops_retrying_when_the_pane_shell_stays_busy`.** Rather than
    adding it to `KNOWN_ENV_FAILURES` (which hides a future regression — the cost the issue
    names), make the script **retry an unexpected failure once in isolation**: a load-contention
    flake passes alone, a real regression fails twice. **Specify the filter form**:
    `docker-check.sh:94` derives the name via `grep -E '^ *FAIL ' | awk '{print $NF}'`, which
    yields a full nextest path — feed it back as `-E "test(=<name>)"` (exact-match form, since
    allowlist matching at `:94-102` is exact string equality and a substring filter would rerun
    siblings). Verify with a fixture test that fails once and passes on retry.
4.5 **`#19` — README fork notice.** `README.md:14` ("Nothing else changes") is false. Replace with
    a short list of the fork's actual changes. **Do not point it at `docs/next/CHANGELOG.md`'s
    `## Unreleased`** — this plan's own constraint forbids fork work from editing that file, so
    the section would not list the features being cited. Inline the list instead. PR body states
    the CLAUDE.md focused-correction exception. Ordering is free (`refs #N` closes at merge, not
    at release), so run it with the 4.2–4.4 wave.

**Wave safety:** 4.1–4.5 touch disjoint paths (`src/persist/` + `src/app/agents.rs`;
`src/config/sidebar.rs`; `src/cli/spec.rs` + `server.rs` + `completion.rs`;
`scripts/docker-check.sh`; `README.md`). The one collision is with **5.2**, which also edits
`src/cli/spec.rs` — safe only because step 5 is serial after step 4.

### 5 — Reshuffle: every move and reorder cell, on every surface

**Three PRs, strictly sequential** — PR-A (5.1–5.4), then PR-B (5.5–5.7), then PR-C (5.8). All
three add `NavigateAction` variants, `palette_id()` arms, `keybind_help` rows, and binding fields
in the same structs, so B and C rebase on their predecessor or conflict.

**Classification for the runtime/client guardrail:** 5.1, 5.3, 5.4, 5.5 are TUI/client
presentation. 5.2, 5.6, 5.7 dispatch existing JSON API methods — no new shared fact, but not
"presentation only" either. 5.8 adds a shared runtime fact and goes through server state.

**Binding fields live in `src/config/model.rs`, not `keybinds.rs`** — `move_tab_previous` at
`:405`, `swap_pane_left..right` at `:429-435` — **and are mirrored again** in the
`Option<BindingConfig>` deserialize struct at `:538+`. Both must stay in sync; nothing enforces it.

5.1 **Workspace reorder from keyboard and palette.** Add `MoveWorkspacePrevious` /
    `MoveWorkspaceNext` variants (`src/app/input/navigate.rs:1849-1858`) dispatching to the
    existing `workspace.move` handler (`src/app/api/workspaces.rs:127`), mirroring
    `move_tab_previous` → `src/app/input/navigate.rs:310`. Add the `model.rs` fields (both
    structs), binding-table rows (`navigate.rs:2084` is the shape), `palette_id()` arms (`:1913`),
    and `help_action_kw` rows in `src/ui/keybind_help.rs`. **Check every existing default binding
    before claiming a key** — collisions here disable an unrelated action rather than erroring.
5.2 **`herdr workspace move` CLI subcommand** at `src/cli/workspace.rs:14-20` — the one capability
    with an API method and no CLI.
5.3 **Register the four `SwapPane*` actions in the palette.** They dispatch
    (`navigate.rs:349`) and bind (`model.rs:429-435`) but have no `help_action` row. Add rows with
    keywords bridging UI wording ("space") to API wording ("workspace"), and **extend the palette
    regression test at `src/ui/palette.rs:524`** to cover them — it guards six other rows and
    would not have caught this.
5.4 **Bindable fields for the five move actions** (`MovePaneToSpace`, `MoveTabToSpace`,
    `MoveTabToNewSpace`, `MovePaneToNewSpace`, `MovePaneToNewTab`) — dispatchable and
    palette-reachable today but with no config field, so unbindable. Ship unbound by default.
5.5 **Context-menu move and reorder entries.** `ContextMenuKind::items()`
    (`src/app/state.rs:1320`) offers none on any unit; dispatch at `src/app/input/modal.rs:883`,
    `:1316`. Reuse the navigator picker for the destination — **a new modal `Mode` needs its own
    mouse-capture branch or clicks leak through to the pane underneath.**
5.6 **Pane drag-move.** `DragTarget` (`src/app/state.rs:1227`) has only `PaneSplit` and
    `PaneScrollbar`. Add a move variant, opening the drag in `src/app/input/mouse.rs` alongside
    the workspace (`:727`) and tab (`:746`) openers, dropping into `pane.move`
    (`src/app/api/panes.rs:624`). Note the API asymmetry: `PaneMoveDestination`
    (`src/api/schema/panes.rs:82`) has no bare `Workspace` variant — a pane dropped on a space
    goes through `NewTab { workspace_id }`.
5.7 **Cross-workspace tab drag.** `tab_drop_index_at` (`src/app/input/mouse.rs:1373`) is gated on
    `on_tab_bar` and `DragTarget::TabReorder` (`src/app/state.rs:1233`) carries a fixed `ws_idx`.
    Widen the drop target to a sidebar workspace row and dispatch the existing
    `TabMoveDestination::Workspace` (`src/api/schema/tabs.rs:63`).
5.8 **Workspace merge.** *Opus lane.* The one genuinely absent capability. Add a
    `workspace.merge` API method beside `workspace.move` (`src/api/schema/workspaces.rs:33`,
    handler at `src/app/api/workspaces.rs:127`), a `herdr workspace merge` CLI subcommand, and a
    `NavigateAction` armed against the navigator picker.
    **The API path is the exposure, and a TUI confirm covers none of it.** The socket is mode
    `0o600` with no network listener, but `HERDR_SOCKET_PATH` is exported into the pane
    environment — every bundled integration asset reads it — so any process in a pane, **including
    the coding agent itself**, can call any method as the owner. Merge gives an agent that just
    read untrusted repository content a one-call primitive that relocates every tab and destroys
    the source.
    **Require the same explicit-intent flag `workspace.close` carries.**
    `src/app/api/workspaces.rs:312-320` refuses to close a workspace with linked worktree
    workspaces unless `close_group=true`. Without the same gate in the same condition,
    `workspace.merge` becomes the way around a control upstream added on purpose (`#3206`). Test it.
    **Save and restore the selection** — `handle_workspace_close` sets `self.state.selected =
    index` before closing (`:328`), so a handler reusing that path yanks the user's view.
    Mirror `ConfirmClose` in the TUI (including its mouse-capture branch) as a second layer, not
    the primary control. **Bump the fork's protocol increment** per step 3.3's stated rule.

### 6 — Release the final tag

Same mechanics as step 2, including the 2.1 gate. **Re-derive the tag from `Cargo.toml` after the
sync** rather than assuming `v0.8.2-palette.5` — step 3 merges upstream, which can bump the
version, and the fork's pipeline dropped the tag-vs-`Cargo.toml` check that would have caught a
mismatch. The convention is `v<upstream-version>-palette.<n>` (`release.yml:8`).

## Panel

Panel: cadence:plan-reviewer, cadence:security-posture-reviewer, cadence:red-team-reviewer ran — 31 findings, 30 folded in, 1 declined

The three seats produced two genuine conflicts, both adjudicated in the plan:

- **`#18` validation placement.** Security said validate the raw string in `restore.rs`; plan
  review said `src/label.rs:7-10` mandates setter-side filters and `valid_agent_name` is private.
  Resolved as setter-side, on raw input, pre-sanitize, with `pub(crate)` visibility — satisfying
  both.
- **`#17`'s site list.** Red team's `grep` found 8 sites; plan review found a 9th
  (`tests/support/mod.rs:18`) that red team's case-sensitive pattern could not match. Folded all
  nine, and the plan's inventory grep is now case-insensitive.

## Panel review findings declined

- **Security's "live-handoff is off the table for this release"** — declined, refuted by red team
  reading the code more precisely. `src/server/handoff.rs:247-250` enforces `expected_protocol`
  only when the flag is supplied, so an omitted flag still hands off across the version change.
  The underlying warning (a hard restart kills every pane) is preserved in Constraints.

## Alternatives declined

- **Sync before landing the PRs** — declined. Both PRs are green now; the sync moves `master`
  under them and forces full re-validation. The sync-first argument was a semantic collision
  between `#26`'s Codex work and upstream's Codex fix — measured and refuted: upstream's Codex
  changes since the merge-base are `distribution/agent-detection/codex.toml` and two SVG icons,
  with no `src/integration/` overlap.
- **One release at the end** — declined by Cameron. The palette work is merged and he is running a
  binary that predates it.
- **Add `#25`'s test to `KNOWN_ENV_FAILURES`** — declined. Cheapest fix, hides a real regression.
- **A `grouped_rows_by_agent` table for `#23`** — declined. Doubles the config surface for one token.
- **Build a move-to-space picker** — declined as already built (the navigator).
- **Deferring pane drag-move, cross-workspace tab drag, and workspace merge** — declined by
  Cameron ("everything").

## Verification

- **Reviews** — every PR carries a review bound to its **head SHA**. A review of an earlier commit
  still renders as complete on the PR page.
- **Review base** — every reviewer dispatch and diff recipe uses `fork/master...HEAD`.
- **New tests actually ran** — derive the expected set from the *test attribute*, not the `fn`
  keyword: `git diff fork/master...HEAD -- <file> | grep -A1 -E '^\+[[:space:]]*#\[(test|tokio::test)\]'`.
  `^\+\s*fn ` cannot match `async fn`, `pub fn`, or `pub(crate) fn` — measured against `#27`, it
  returns 1 name and misses both new tests, passing while nothing was verified.
- **CI** — `gh pr checks <n> -R cameronsjo/herdr` green on all five checks.
- **Sync** — `bash <sync-worktree-abs-path>/scripts/docker-check.sh > /tmp/dc.log 2>&1; echo $?`
  returns `0` with no `unexpected test failure(s)` line, and `/tmp/dc.log` names the merge commit.
- **`#17`** — `grep -rni 'protocol.*\b(21|22)\b' tests/ docs/next/api/` returns nothing stale after
  the change; the container run passes `tests/cli/sessions.rs`, `tests/api_ping.rs`.
- **`#18`** — the raw ZWSP name is rejected at the setter boundary; the rejected pane still reports
  `managed_agent` active with no name; a second identical name is dropped by the dedupe. Each must
  fail against unmodified code.
- **`#23`** — a `rows_by_agent` override under `group_by = "workspace"` does not print the
  workspace token twice.
- **`#24`** — `herdr server --help` lists `live-handoff`; `src/cli/spec.rs:1079` passes.
- **`#25`** — a fixture failing once and passing on retry exercises the retry path.
- **Reshuffle** — `src/ui/palette.rs:524` covers the swap and workspace-move rows; `herdr
  workspace move` moves a space; `workspace.merge` refuses a linked-worktree group without the
  explicit flag.
- **Releases** — `git rev-list -n1 <tag>` equals `fork/master`; the tag matches `Cargo.toml`; the
  tap formula carries the new version; `brew upgrade herdr` lands it.
- **Ledger** — every cell filled from an artifact (`git ls-remote`, `gh pr view`), never an agent's
  self-report. Close with `closed N · filed M · net N−M`.

## Provenance

Every commit carries the producer tuple, `refs #<issue-number>` in the body, no closing keywords.
