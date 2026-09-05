---
status: "done"
updated: "2026-09-05"
branch: "master"
body_sha256: "5362f5e19bf1021c2e1e945cd6de44e18a89d0c130623883457ef57fbd3382e5"
session: "keen-anvil"
session_id: "f5b59d2d-332a-4fa6-b3fb-905f5687fc01"
model: "claude-opus-5"
harness: "claude-code 2.1.247"
machine: "cf6e768835c7"
approved_in: "calm-bellows"
approved_session_id: "3589c9aa-4396-4bfd-8f34-be032aa80b8c"
---

# Sync herdr fork with upstream, then cut v0.8.2-palette.3

## Context

`~/Projects/cadence-ecosystem/herdr` is Cameron's fork (`fork` = `cameronsjo/herdr`) of `herdrdev/herdr` (`origin`). The last sync was 2026-08-23 (`v0.8.2-palette.2`). Since then upstream landed **8 commits** that the fork does not have; the fork sits 70 commits ahead with its own palette/command-palette/space-move work.

Goal: land those 8 upstream commits on `fork/master`, validated, then cut `v0.8.2-palette.3` and `brew upgrade` on sjomba. **Not** in scope: restarting the M5's `fleet` herdr server.

Runbook this follows: `~/.claude/docs/herdr-upgrade.md`. Every write targets `fork`, never `origin`.

## Measured starting state

- `fork/master` == local `master` == `8038308d`; working tree clean; no extra worktrees.
- Behind `origin/master` by 8, ahead by 70. Merge base `d6dae883`.
- 22 files touched by **both** sides since the merge base — real conflict risk, concentrated in `src/app/input/modal.rs`, `src/app/runtime_mutations.rs`, `src/cli/runtime.rs`, `src/app/api/workspaces.rs`, `src/protocol/wire.rs`, `docs/next/CHANGELOG.md`.
- `docs/next/CHANGELOG.md` is the **union** shape: upstream has cut no release since `0.8.2`, so both sides only appended under `## Unreleased`. Keep both sides; do not take `--theirs`.
- Upstream bumps `PROTOCOL_VERSION` 20 → 21 in `src/protocol/wire.rs`. Consequence noted below.
- Docker 28.4.0 is up; `docker-check.sh` can run. `just check` cannot (Zig/macOS SDK wall).
- No herdr server running on sjomba. `herdr --version` = 0.8.2. `fork/master` has no branch protection, so the script's direct push works.

## Steps

1. **Run the sync script from the primary checkout.**
   `bash scripts/sync-upstream.sh` — fetches both remotes, creates `../herdr-worktrees/sync-upstream-20260826` off `fork/master`, merges `origin/master`, auto-resolves the `README.md` conflict, and stops on anything else. It is resumable.

2. **Resolve conflicts by hand in that worktree, then re-run the same script** to land the merge commit.
   - `docs/next/CHANGELOG.md`: union — keep both sides' `## Unreleased` entries. Verify by counting `- ` lines contributed by each side.
   - Rust conflicts: resolve **toward upstream's refactor** and re-express the fork's addition in the new style. Do not preserve the fork's old form as an exception.
   - Watch `src/cli/runtime.rs` / `src/app/api/workspaces.rs` against upstream #3206 (*require explicit workspace group close*) — the fork's move-pane/move-tab-to-space work lives in the same surface.

3. **Validate from the worktree** (not the primary checkout — the script derives its root from `BASH_SOURCE` and would print a PASS on the pre-merge tree):
   `bash scripts/docker-check.sh`
   Read only the `unexpected test failure(s)` line. A clean auto-merge is not a safe merge — the 2026-08-13 sync merged `src/app/input/modal.rs` with zero markers and still broke; `-D warnings` caught it. If an unexpected failure appears, re-run that one test in isolation on pristine `fork/master` and on the branch with `-E "test(<name>)"` before attributing it to the merge.

4. **Push and fast-forward** (script prints these):
   - `git -C <worktree> push --no-follow-tags fork sync-upstream-20260826:master`
   - `git merge --ff-only sync-upstream-20260826` in the primary checkout, *before* deleting the branch.
   - `git worktree remove <worktree> && git branch -d sync-upstream-20260826`

5. **Cut `v0.8.2-palette.3`.** `Cargo.toml` `version` stays `0.8.2` — the palette suffix lives only in the tag.
   ```
   git tag -a v0.8.2-palette.3 -m "v0.8.2-palette.3

   - <one line per user-visible change from the 8 upstream commits>"
   git push --no-follow-tags fork refs/tags/v0.8.2-palette.3
   ```
   `--no-follow-tags` is mandatory: `push.followTags` is global and the sync just imported upstream's annotated tags; pushing one of those fires `release.yml` against an upstream version.

6. **Watch `release.yml`, then upgrade.** `gh run watch -R cameronsjo/herdr`, then `brew update && brew upgrade herdr`.

7. **Verify by provenance**, not by grepping the binary:
   - `git merge-base --is-ancestor <merge-sha> v0.8.2-palette.3 && echo YES`
   - `git rev-list -n1 v0.8.2-palette.3` equals `fork/master`
   - tap formula version: `gh api repos/cameronsjo/homebrew-tap/contents/Formula/herdr.rb --jq .content | base64 -d | grep version`
   - `herdr --version` on sjomba

## Heads up on PROTOCOL_VERSION

`check_client_version` requires an **exact** match. Once sjomba's binary is 21, a client from it cannot attach to the M5's `fleet` server while that server still runs the 20 binary — `bash scripts/observatory.sh` will refuse. That is a mismatch error, not a broken observatory. Clearing it needs an M5 fleet restart (kills every pane, ~10 live sessions) or the unproven `herdr server live-handoff --import-exe`. Out of scope here; flagged so the refusal is legible when it happens.

## Verification

`docker-check: PASS` with no `unexpected test failure(s)` line is the gate on the merge. The release is verified by the four provenance checks in step 7. No behavioral change is being authored, so there is nothing new to test beyond upstream's own suite.

## Panel

Panel: none — mechanical execution of an existing, verified runbook (`~/.claude/docs/herdr-upgrade.md`); no design decisions, no new code, no security-control posture change.

## Alternatives declined

- **Merge and stop, no release** — Cameron chose to cut the tag in the same pass (AskUserQuestion, 2026-08-26).
- **Full path including an M5 fleet restart** — declined for the same reason it always is: the restart kills every live pane.
- **`just check` for validation** — impossible on this Mac (Zig 0.15.2 vs macOS 26 SDK); `docker-check.sh` is the substitute.

---

## Execution record — COMPLETE 2026-08-26

All 7 steps done.

- Merge commit: `407c6e85` (amended from `0cec2bca` to fix rustfmt wrapping), on `fork/master`.
- Conflicts resolved (3, all unions):
  - `docs/next/CHANGELOG.md` — fork's 3 `### Fixed` entries kept alongside upstream's 1 (#2711).
  - `src/app/runtime_mutations.rs` — import list: fork's `PaneMoveParams` + upstream's `WorkspaceCloseParams`.
  - `src/cli/runtime.rs` — import list: fork's `PaneMoveParams`/`TabMoveParams` + upstream's `WorkspaceCloseParams`.
  - The #3206 bodies auto-merged clean against the fork's move-pane work; `cargo clippy -D warnings` found nothing.
- Validation: `docker-check: PASS`, exit 0, no `unexpected test failure(s)` line. The first run failed on
  `cargo fmt --check` only (hand-wrapped import list); no test regressions in either run.
- Tag `v0.8.2-palette.3` pushed with `--no-follow-tags`; `Release` run 33032703152 green
  (build / release / tap-bump all success).
- Provenance: merge sha is an ancestor of the tag; `git rev-list -n1 v0.8.2-palette.3` == `fork/master` ==
  `407c6e85f9ef942397116c7bd9c302c7ecea4afa`; tap formula `version "0.8.2-palette.3"`;
  `brew upgrade herdr` 0.8.2-palette.2 -> 0.8.2-palette.3. `herdr --version` reports `0.8.2` as designed
  (Cargo.toml version is unsuffixed; the palette suffix lives only in the tag and the tap).

### Deviations

- Amended the merge commit (unpushed at the time) to fix `cargo fmt --check` on `src/cli/runtime.rs`,
  rather than adding a follow-up commit. No local `cargo`/`rustfmt` on this Mac, so the container's
  fmt check was the first place the wrapping could be measured.

### Notes

- `PROTOCOL_VERSION` is now 21 on sjomba. The M5 `fleet` server still runs the 20 binary, so
  `bash scripts/observatory.sh` will refuse with a version mismatch until that server is restarted.
  Out of scope here, as planned.
- The `Close pending-release issues` workflow failed on this push (exit 4, missing `GH_TOKEN`).
  Pre-existing since 2026-08-23, unrelated to this merge, and arguably correct behavior on a fork —
  it scans pushed commits for `refs #<issue>` and would otherwise close *upstream* issues.
