# Fork divergence ledger

Every place `cameronsjo/herdr` (`fork`) has diverged from `herdrdev/herdr`
(`origin`), newest first, grouped by the plan that landed it. Every fork PR
adds its entry to the top of the relevant section (or a new section) when it
merges. Every upstream sync re-runs the regression checks listed below for
every section still in the fork, before the sync merge is pushed.

## Upstream syncs

One line each — these replace no fork behavior, they just pull upstream
forward. Full history: `git log --oneline --merges origin/master..HEAD`.

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

## Keep SIGPIPE ignored inside the unit-test harness (issue [#39](https://github.com/cameronsjo/herdr/issues/39))

### fix(platform): keep sigpipe ignored inside the unit-test harness

- **PR:** [cameronsjo/herdr#75](https://github.com/cameronsjo/herdr/pull/75)
- **Files:** `src/platform/unix_common.rs`
- **Replaces:** The issue's premise — a pty write losing its reader — was wrong. `begin_cli_output()` flips SIGPIPE to `SIG_DFL` so a piped `herdr --help | head` dies quietly on purpose; correct for the shipped CLI. `cargo test --bin herdr` runs every test in one process, so once a test that prints and then drops a reading peer calls it, SIGPIPE stays fatal for every later write-to-closed-pipe in the same run, and the suite exits 141 partway through. Which test dies moves between runs because the process-wide disposition, not any one test, is what changed. `cargo nextest run` hides this because it isolates each test in its own process. Restoring the disposition inside just the one offending test was rejected: it silently re-breaks the next time any test exercises a CLI print path. The fix gates `begin_cli_output`/`end_cli_output`'s bodies on `#[cfg(not(test))]` so the test harness never touches the process-wide disposition.
- **Regression check:** `cargo test --bin herdr > log 2>&1; echo $?` — prints a `test result:` line and no longer exits 141.

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
- **Regression check:** `herdr workspace merge --help` lists the command; `cargo test workspace::merge`.

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
- **Regression check:** launch a Codex agent pane and confirm the title reflects live state; `cargo test integration::codex`.

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
- **Regression check:** `cargo test config::sidebar::grouped_rows`.

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
