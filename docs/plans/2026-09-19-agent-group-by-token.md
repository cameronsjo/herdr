---
updated: "2026-09-19"
branch: "master"
body_sha256: "e577bac5ae9b479638f3cf418a6a69c126bb99431e6fbeb3deb37d44b8d2139e"
session_id: "5a79cc89-1a16-4c7c-b85e-d6c89dd6982a"
model: "claude-opus-5"
harness: "claude-code 2.1.278"
machine: "cf6e768835c7"
approved_session_id: "cb45fd0a-6942-4744-a704-f57c90bd4ada"
status: in-progress
tier: T1
---

# herdr sidebar: make herdr-projects threads read as one project

## Context

herdr-projects gives each thread its own worktree workspace. The fork groups agent rows **per workspace**, so each thread becomes its own top-level group, and rows show only Claude's auto-generated session title. Brief: `/tmp/herdr-sidebar-brief.md`. Live state was read on 2026-09-19 (`herdr workspace list`, `herdr agent list`, `herdr-projects overview homelab`). Nothing was changed.

**Live facts behind the diagnosis**
- herdr-projects puts `project`, `thread`, `rank`, and `review` tokens on each **pane** (`pane report-metadata --ttl-ms 300000`, plugin `src/herdr.rs:375`, built at `src/threads.rs:55-61`). It also sets agent `name` = `hp-homelab-t-000N` (`src/thread.rs:188`).
- Thread **workspaces** get only a label, which is the thread title (`worktree create --label <title>`, plugin `src/herdr.rs:285`). They get no workspace tokens.
- The fork CLI already has `herdr workspace report-metadata`. Spaces rows can show `$name` workspace tokens (`src/ui/sidebar/tokens.rs:163-167`).

## Findings per item

| # | Root cause | Fix belongs in |
|---|---|---|
| 1 Threads are top-level groups | Grouping key is `(scope, agent.workspace_id)` (`src/client/shell/agent_sidebar.rs:448`). The header is `workspace.label` (`:345-349`), gated by `agent_grouping_is_effective` (`:67-74`). This is fork code (PR #11 `d6a402e0`). One workspace per thread means one group per thread. | **Fork** |
| 2 Inconsistent agent names, no t-id | `~/.config/herdr/config.toml:105-107` sets `grouped_rows = [[state_icon, terminal_title_stripped]]`, so rows show only Claude's session title. "Brief for homelab-t-0002" was Claude's auto-title from the one-line prompt `Read .herdr-project/…/brief.md` (plugin `src/thread.rs:200`). It has since changed to `speaches-model-install-runbook`. The `name` (`hp-homelab-t-0002`) and `$thread` token exist but no row references them. | **Config** |
| 3 Thread worktrees filed under repo spaces | Spaces nests linked worktrees under the non-linked workspace with the same `repo_key` (`src/client/shell/sidebar.rs:438-540`). The thread worktrees are real linked worktrees of `homelab` and `rss-pipeline`, so this is correct repo semantics. Nothing tells Spaces which project a thread belongs to. | **Upstream** (herdr-projects sets a workspace token) + **config** (`$project` on Spaces rows). No grouping change recommended. |
| 4a Two "Projects" workspaces | `w6X` and `w74` have no custom name and a root cwd of `~/Projects` (not a repo). The auto-label falls back to the cwd basename (`src/workspace.rs:1154-1174`, `src/workspace/git/discovery.rs:40-53`). Not a bug. | **User action**: `herdr workspace rename` (Cameron; not done here per brief) |
| 4b Bare `○` rows | Same config line as #2. `w6R:pP`, `w6R:pF`, and `w6X:p1` have no OSC terminal title, so `terminal_title_stripped` is `None`. The row keeps only `state_icon` (`src/ui/sidebar/tokens.rs:76-121`). Row tokens have no "first non-empty of" fallback (`AgentSidebarToken`, `src/config/sidebar.rs:117-133`). | **Config** now; optional **fork** fallback token |
| 5 No visible way to clear `focus` | `agent.view.set` shows the view label in the header's accent color, but it zeroes the header's click target (`src/client/shell/agent_sidebar.rs:205-234`, `:216`). No palette entry, keybind, or CLI verb clears a view; only the `agent.view.clear` API does. herdr-projects does ship a plugin action, **"Projects: clear sidebar focus"** (`herdr-plugin.toml:38-42`), in herdr's workspace action menu. It is easy to miss. | **Fork** (make the active-view label clear the view on click, plus a palette entry) |

Also stale: the comment at `~/.config/herdr/config.toml:117-123` says `rows_by_agent` beats `grouped_rows`. Since fork `b82258cf`, `rows_for_agent` returns `grouped_rows` whenever grouping is on (`src/config/sidebar.rs:469-476`). The `claude` override, including `state_text` and `$claude_account`, has no effect today.

## Grouping key today and what project-aware grouping needs

- **Today:** `AgentGroupBy::{None, Workspace}` (`src/config/sidebar.rs:440-446`). The key is `workspace_id` and the header is the workspace label.
- **Needed:** a token-keyed mode, `group_by = { token = "project" }`.
  - The key is the pane token's value when present, else the workspace, so untokened agents keep today's behavior.
  - The header is the token value (`homelab`).
  - The data is already on the client: `ClientShellAgent.tokens` is used at `agent_sidebar.rs:418`.
  - `AgentsSidebarConfig` is client-local and not part of the wire codec, so the stable-endpoint rules don't apply.
- **Contiguity:** members of one project are not always adjacent. In today's order `w75:p1` (homelab) is followed by `w75:p2` (untokened), then `w77`/`w79` (homelab).
  - The current gate would switch grouping off.
  - The mode therefore needs a stable gather: each key's rows move up behind the key's first appearance.
  - The gather must happen in `ordered_agent_pane_ids` (`agent_sidebar.rs:103`) so rendering, hit-testing, scrolling, and agent navigation use one order.
- **Under `focus`:** the plugin's view filters to `project=homelab` and sorts by `rank`. All rows then share one key, so the view renders as one `homelab` group. Today the rank sort interleaves workspaces and switches grouping off.
- **Token expiry:** the tokens have a 300 s TTL that the plugin ticker refreshes. If the ticker stops, the tokens expire and rows fall back to per-workspace groups. That fallback is safe.

## Fix plan (ordered, fork first)

1. **Fork: token-keyed agent grouping.** Branch `feat/agent-group-by-token` in a worktree off `fork/master`.
   - Extend `AgentGroupBy` with `Token(String)` and deserialize both `"workspace"` and `{ token = "…" }`.
   - Add one pure `agent_group_key(agent, group_by) -> (label, key)` and use it from `agent_grouping_is_effective`, the header decision (`:345`), and `group_key` (`:448`).
   - Add the stable gather to `ordered_agent_pane_ids`. Mirror the endpoint list (`endpoint_agents.rs:127`) if it shares the path.
   - Tests in `src/client/shell/tests/agents_worktrees_notifications.rs`: tokened agents in separate workspaces share one header; untokened agents keep workspace headers; interleaved input is gathered, and the hit-test order matches render order; a filter plus rank-sort view stays grouped.
   - Add a `docs/fork/CHANGES.md` entry and a `docs/next` sidebar-config doc line.
   - The per-row work is a hash lookup on data the row already builds, so no allocation is added to the render loop. Run `just bench-render-scale` because this changes a pane-scaled path.
2. **Fork: clear the view from the TUI.**
   - While a view is active, make the header's view label a click target that sends `agent.view.clear`. Today `:216` zeroes that target.
   - Add a palette entry "Clear agent view" through `src/ui/keybind_help.rs`.
   - This uses the existing JSON API method, so there is no private-socket-only behavior.
   - Test: the click and the palette entry both clear the view; the sort toggle still works when no view is set.
3. **Config (Cameron's dotfiles), independent of 1 and 2 and can go first:**
   - `grouped_rows = [["state_icon", "$thread", { token = "terminal_title_stripped", … }], [{ token = "agent", dim = true }]]`. The second row prints `hp-homelab-t-0001`, or `claude` for unnamed agents, and a bare `○` row gets a readable second line.
   - Set `group_by = { token = "project" }` once step 1 ships.
   - Correct the stale `rows_by_agent` comment.
   - Edit via the chezmoi source, then run `herdr server reload-config` (no restart).
4. **Optional fork: fallback token** (e.g. `{ first_of = ["terminal_title_stripped", "agent"] }`) so a row can show one label instead of two rows. Do this only if step 3's two-row layout feels noisy.
5. **Upstream ask (herdr-projects), only after 1–3:** a small issue asking `thread start` and the ticker to also send `workspace report-metadata --token project=<slug> --token thread=<id>` on thread workspaces. Spaces rows could then show `$project`. Filing it needs Cameron's go-ahead; it is a third-party repo.
6. **Cameron, whenever:** rename `w6X` or `w74` so the two "Projects" spaces differ.

## Amendment: brief 2 (2026-09-19, during execution)

Source: `/tmp/herdr-sidebar-brief-2.md`. Live state re-read with `herdr agent list` and `herdr pane list` (read-only). Pending Cameron's go-ahead on the new fork scope (7, 8).

| Brief item | Finding (file:line, live evidence) | Fix belongs in |
|---|---|---|
| 4 Focus indicator, no clear | An indicator exists but is weak: the view label (or `filtered` when unlabeled, `src/server/client_shell.rs:161-165`) replaces `grouped` at the right end of the `agents` header in the accent color (`src/client/shell/agent_sidebar.rs`, `render_agent_panel_header`). Nothing clears it from the TUI (step 2 above). The global menu (`src/client/shell/global_menu.rs`) has only settings, keybinds, reload config, what's new, detach. | **Fork** — widen step 2 |
| 5 Nest threads under the project; `t-NNNN · title` rows | Step 1 gives one `homelab` header. **New:** the thread panes `w77:p1` and `w79:p1` now carry **no tokens**, only the coordinator `w75:p1` keeps `project=homelab`. The 300 s pane-token TTL lapsed after the threads resolved, so the rows fall back to per-workspace groups. The row label is config: tokens already join with ` · ` (`src/ui/sidebar/tokens.rs:181-187`), so `["state_icon", "$thread", "workspace"]` renders `t-0001 · <thread title>`, truncated to width. | **Fork** (step 1 also reads workspace tokens) + **upstream** (workspace tokens, step 5) + **config** |
| 6 Blocked agents sink | No within-group ordering exists. `priority` sort puts blocked first but turns grouping off (`agent_grouping_is_effective`). | **Fork** (new option) |
| 7 Idle shell rows | **Not reproduced.** `pane_details` skips a pane with no agent name and no detected agent (`src/workspace/aggregate.rs:33`, since `207be3c7`). Live: the shells `w77:p2`, `w77:p3`, `w79:p2` appear in `pane list` and not in `agent list`. The 12 live agent rows are all Claude panes. The extra rows Cameron saw are more likely the three untitled Claude panes that render as a bare `○` (finding 4b). | **Config** (step 3 gives them a second line); ask Cameron for a screenshot if rows remain |
| Name shown | The fork's `agent` token resolves `display_agent → name → agent → title` (`agent_sidebar.rs`, `agent_label`). The live `grouped_rows` shows only `terminal_title_stripped` (`~/.config/herdr/config.toml:105-107`), which is why `hp-homelab-coordinator` reads as `homelab-project-setup`. No fork change; step 3 adds the `agent` token. | **Config** |

**Added fix steps**

7. **Fork: workspace-token fallback for token grouping.** `agent_group_key` checks the pane's token first, then the workspace's token (`ClientShellWorkspace.tokens`), then the workspace. Once herdr-projects sets `project` on thread workspaces (step 5), grouping survives the pane-token TTL. Test: a pane without the token in a workspace with it joins the token group.
8. **Fork: `blocked_first` agent option.** `[ui.sidebar.agents] blocked_first = true` moves blocked agents to the top of each group with a stable sort after the gather. Runs stay contiguous, so grouping stays on. Without grouping it moves them to the top of the list. Default `false`. This covers "Needs you" without a synthetic group that would break the one-key-per-run rule. Test: blocked member leads its run; the order of other members is unchanged.
9. **Step 2 widened.** Render the active view as `<label> ✕` in the accent color. Clicking it sends `agent.view.clear`. Add "clear agent view" to the global menu while a view is active. Add a palette row, which needs a new `KeybindAction::ClearAgentView` with an unbound `keys.clear_agent_view` (the palette's core rows come from `KeybindAction::palette_id`, `src/input/keybindings.rs:83`; `src/ui/keybind_help.rs` no longer exists).
10. **Config, extends step 3:** `grouped_rows = [["state_icon", "$thread", "workspace"], [{ token = "agent", dim = true }, { token = "terminal_title_stripped", dim = true }]]`, and `blocked_first = true` after step 8 ships.

**Release note:** steps 1 and 2 change the binary, so they need a fork release and a server restart, which kills live panes (`herdr-restart-semantics`). Batch them into the next palette release; don't ship each separately.

## Alternatives declined

- **Upstream-only fix** (herdr-projects creates thread tabs inside the coordinator workspace): plugin tab threads already exist (`src/threads.rs:262`), but repo threads need their own worktree cwd. That would also take away per-thread workspace focus. Declined.
- **Group Spaces by project:** it would break the repo/worktree tree, which is correct, and needs the upstream workspace token first. Declined; a `$project` row token covers it.
- **Header uses the coordinator workspace's label ("Homelab") instead of the token value:** this needs a cross-agent lookup per frame for a capitalization gain. Deferred.

## Verification

- Step 1/2: run `just check` in the worktree. For a live check, use a `herdr-dev` debug server (`env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo run -- …`) with two workspaces whose panes report `project=x` through `herdr pane report-metadata`. Confirm one `x` header. Then set a view through the socket and confirm clicking the label clears it. Never test against the live `fleet`/default server.
- Step 3: after `herdr server reload-config`, the thread rows show `t-0001`/`t-0002` plus `hp-homelab-t-000N`, and the three untitled Claude panes show `claude` instead of a bare `○`.

## Global Constraints

- Never close, move, rename, or restart a live pane, workspace, or the herdr server. Test only against a `herdr-dev` debug server.
- All writes target `fork` (`cameronsjo/herdr`). Writing to `herdrdev/herdr` or `eliasstravik/herdr-projects` needs Cameron's approval each time.
- Commit style: lowercase conventional commits, no closing keywords. Get Cameron's alignment on the message before committing (repo CLAUDE.md).

## Deviations

- Step 1: the `CHANGES.md` ledger entry moves to the end of the branch, as one section for the PR, because its heading carries the PR number.
- Step 1: only token mode gathers. `group_by = "workspace"` keeps its existing order and contiguity check, so no current layout changes.
- Step 1: the stable gather also runs in the multi-endpoint list (`aggregate_navigation.rs`), keyed by machine plus group, because an active view sorts rows across endpoints.
- Step 1: `group_by = { token = "$project" }` is accepted as well as `"project"`, since rows spell the same token with `$`.
- Step 1: fixed the stale `grouped_rows` reference entry, which said `rows_by_agent` wins while grouped.
- Step 2: `src/ui/keybind_help.rs` no longer exists. Palette core rows come from `KeybindAction::palette_id`, so the palette row needs a new `KeybindAction` (amendment step 9).
- Step 2: the TUI endpoint accepts only advertised methods, and `agent.view.clear` was not one. It joins `CLIENT_SHELL_METHODS` with a newly recorded shape digest (the fork's `workspace.open` precedent, PR #64); no existing digest changes.
- Step 8: the ordering functions take `&AgentsSidebarConfig` instead of `&AgentGroupBy`, since they read `blocked_first` too.
- Review: token mode skips the contiguity scan, since the gather guarantees it and the scan is quadratic with many one-agent groups. Under a view with several token groups, each group takes its best-ranked member's place; a test now pins that.
- Bench (`just bench-render-scale`, 15 panes, median µs, client composition): background 348 off / 313 workspace / 322 token+`blocked_first`; active 307 / 309 / 303. The fork bench gained a token arm that reports no tokens, so every key takes the workspace fallback scan.
- Polish: fixed `blocked_first` doing nothing under an interleaving view with workspace grouping (it now leads the whole list, as documented), reused `group_header` in the endpoint list, and made view labels drop format characters (security arm Nit, pre-existing).
- Build env: CommandLineTools is gone from sjomba, and the vendored libghostty-vt needs Zig 0.16. Local builds now use the Xcode SDK plus `mise install zig@0.16.0`.

## Orchestrator

Driver: opus — one session implements in sequence; no fan-out (fewer than 3 independent slices).

## Tasks

- [x] 1. Fork: token-keyed agent grouping + gather in `ordered_agent_pane_ids` + tests + `CHANGES.md`
- [x] 2. Fork: clear an active agent view from the header label and the palette
- [ ] 3. Config: `grouped_rows` with `$thread` and `agent`, `group_by = { token = "project" }` after step 1, fix the stale comment
- [ ] 4. Optional fork: `first_of` fallback token
- [ ] 5. Upstream herdr-projects issue for workspace tokens (only with Cameron's go-ahead)
- [ ] 6. Cameron: rename one of the two "Projects" workspaces
- [x] 7. Fork: workspace-token fallback in `agent_group_key` (brief 2, item 5)
- [x] 8. Fork: `blocked_first` option (brief 2, item 6)
- [x] 9. Fork: widen step 2 — `✕` on the view label, global-menu entry, palette row (brief 2, item 4)
- [ ] 10. Config: `$thread · workspace` row plus `agent`, `blocked_first` (brief 2, items 5 and name)

Panel: none — T1 investigation plus a client-local sidebar presentation change; no security-critical control touched.
