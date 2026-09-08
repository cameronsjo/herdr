---
status: "in-flight"
updated: "2026-09-08"
branch: "master"
body_sha256: "acdde59b2200dc179b264397148404b25d588443f2397991dd08e5d93a2f674e"
session: "fern-mallet"
session_id: "55259bb0-df0c-4587-9f13-89894903c400"
machine: "cf6e768835c7"
approved_in: "frost-lantern"
approved_session_id: "221fb45e-f1d3-4262-b275-df9dd396ab8e"
---

# Land the upstream sync, cut the release, fit CI to the fork

## Context

Cameron's Homebrew tap installs `herdr 0.8.2` while the fork's tree is at upstream `v0.9.0`. A prior Codex session merged upstream (`e839cf59`, fork PR #56) and stopped there — it did the sync, not the release. Releases are tag-triggered (`.github/workflows/release.yml`, `on: push: tags: v*`) and no `v0.9.0-palette.*` tag exists, so no build ran, no asset was published, and the tap's `update-formula` receiver had nothing to bump.

Upstream then moved 3 commits further. Those are merged in a worktree already (`f8fe3131`, clean).

Cameron also asked to fit CI to what this fork actually is: a public fork carrying a command palette upstream rejected, shipping **one** artifact — an arm64 macOS binary via `cameronsjo/homebrew-tap`. Today a full CI run spends 21.8 minutes of runner time, 12.8 of it on Windows targets the fork has never shipped, and a weekly `Sync upstream` cron has failed 5 for 5 since 08-10.

Outcome: tap current at `0.9.0-palette.1`, a release procedure that survives the session, and CI shaped to the one thing the fork ships.

## Panel

Panel: `cadence:plan-reviewer`, `cadence:operability-reviewer` ran — 21 findings, 18 folded in, 3 declined.

The two that changed the plan's shape:

- **A reused tag corrupts every installed machine.** `softprops/action-gh-release@v3` (`release.yml:122`) updates an existing release in place and overwrites the asset. Delete-and-re-push a tag and the new binary has a new sha256 while the tap formula is still pinned to the old one — `brew upgrade` then fails a checksum everywhere, with an error that reads like a corrupted download. A palette tag is immutable; a bad build is fixed by cutting the next counter.
- **A leaked upstream tag is a silent downgrade, not a confusion.** `release.yml` triggers on `v*` and `fork_release_notes.py:21` makes the palette segment *optional*, so a followed `v0.9.0` publishes a real fork release and dispatches `version=0.9.0` to the tap. `0.9.0` sorts **above** `0.9.0-palette.1`, so brew pins the palette-less build and never upgrades back.

Declined: adding a `concurrency:` group to `release.yml` (blast radius is a confusing run list; the tap receiver already serializes), adding release-failure notification (releases are operator-watched — revisit when hermes drives the tag), and keeping the cron green-when-conflicted (Cameron ruled the cron out; drift detection moves to a script instead).

## Alternatives declined

- **Widen `FORK_OWNED` in `scripts/sync-upstream-ci.sh` so weekly syncs auto-resolve.** Trades a loud red for a quiet wrong merge. That allowlist means "upstream's version of this file is never what we want", and the palette carry is exactly where a silently-dropped upstream fix would hurt.
- **Delete the fork-irrelevant upstream workflows.** A deleted file becomes a recurring modify/delete conflict on every sync. The fork already established the cheaper pattern — `if: github.repository == 'herdrdev/herdr'` on four workflows. Follow it.
- **Cut CI to macOS only.** Cameron chose Linux + macOS; ubuntu is 3.7 min and catches most portability regressions a sync could introduce.
- **Clean up `distribution/*.json`, `docs/versions/`, `.github/MAINTAINERS`, `APPROVED_CONTRIBUTORS`.** All describe upstream's world, none is read at build time, every edit is a future conflict. Loose ends, not work.

---

## Task 1 — Land the upstream sync

The merge exists at `../herdr-worktrees/sync-upstream-20260908` (`f8fe3131`), 4 commits ahead of `fork/master`, clean. Upstream's three: `68c7b78e` worktree-group collapse, `ffa0892e` marketplace worker moved out, `9e01168b` ssh setup errors.

- [x] Validate from **inside the worktree** — `scripts/docker-check.sh` mounts the tree it lives in, so the primary checkout's copy would test the pre-merge tree and still print PASS.
  ```
  cd ../herdr-worktrees/sync-upstream-20260908
  PATH="$HOME/.cargo/bin:$PATH" DEVELOPER_DIR=/Library/Developer/CommandLineTools \
    ZIG="$HOME/.local/share/mise/installs/zig/0.15.2/bin/zig" just check
  ```
  Prerequisites beyond that env line, because `just check` → `ci` → `integration-assets-test` and `docs-contract-test` shell out to **`bun`**, and `windows-lint` runs `rustup target add x86_64-pc-windows-msvc`. Confirm `command -v bun` first. A missing prerequisite fails for a reason unrelated to the merge; the fallback is `./scripts/docker-check.sh` from the worktree (~12 min).
- [x] Push and open the PR against `fork`:
  ```
  git -C ../herdr-worktrees/sync-upstream-20260908 push --no-follow-tags -u fork sync-upstream-20260908
  ```
  `--no-follow-tags` is mandatory — `push.followTags` is global and the sync fetch imported upstream's new annotated tags, `v0.9.0` among them. See the Panel note on what a leaked tag does.
- [x] PR title must satisfy `ci.yml`'s `conventional-commits` job, which validates the **title** on `pull_request`: `chore(sync): merge upstream through 9e01168b`.
- [x] Watch `gh pr checks --watch`, then merge on Cameron's go **with a merge commit, not a squash** — a squash rewrites the SHA, so the local branch stops being an ancestor and the fast-forward below cannot work.
- [x] Fast-forward the shared checkout from `fork/master` (not from the local branch), **before** deleting anything — `git branch -d` compares against HEAD and refuses otherwise:
  ```
  git -C . fetch fork && git -C . merge --ff-only fork/master
  git worktree remove ../herdr-worktrees/sync-upstream-20260908
  git branch -d sync-upstream-20260908 && git push fork --delete --no-follow-tags sync-upstream-20260908
  ```
  `--no-follow-tags` on the delete too: `followTags` is still evaluated on a delete-only push.

## Task 2 — Cut the release, as a committed script

The panel's strongest structural finding: the release procedure as plan prose dies with the session, and every step is mechanical with exactly one correct outcome. It becomes `scripts/cut-fork-release.sh <tag>`, committed, so the next release is one command and a future hermes auto-maintainer has something to call.

- [x] Establish the counter from the record rather than assuming. `release.yml:29` writes a historical tag as `0.8.0-palette1` — **dotless**, which today's `TAG_RE` rejects — so the shape is not self-evident:
  ```
  gh release list --repo cameronsjo/herdr --limit 10
  ```
  Expectation to confirm, not assert: the counter resets per upstream version, making the next tag **`v0.9.0-palette.1`**. `Cargo.toml` is already `0.9.0`; no version bump.
- [x] Confirm the release credentials exist **before** tagging. `tap-bump` runs after `release`, so a missing credential fails *after* the GitHub release is published, and the only clean recovery is a new tag:
  ```
  gh variable list --repo cameronsjo/herdr | grep RELEASE_APP_ID
  gh secret list   --repo cameronsjo/herdr | grep RELEASE_APP_PRIVATE_KEY
  ```
- [x] Write `scripts/cut-fork-release.sh <tag>`, failing closed at each step before any push:
  1. Assert tag shape against `fork_release_notes.py`'s regex **with the palette segment required** — the script is stricter than the validator on purpose.
  2. `git fetch fork`; assert the working tree is clean and the tag is unused locally and on `fork`.
  3. Assert no upstream tag has leaked, as a verdict rather than 80 lines to eyeball:
     ```
     git ls-remote --tags fork | grep 'refs/tags/v' | grep -v -- '-palette\.' | grep -qE 'v0\.(8\.2|9\.[0-9]+)$' && echo LEAK || echo CLEAN
     ```
  4. Tag **`fork/master` explicitly**, never HEAD — `git tag -a "$TAG" fork/master -m …`. The session may be sitting in a worktree or on another branch, and a tag one commit off ships a binary that looks right and is not.
  5. Prove it landed where intended: `test "$(git rev-parse "$TAG^{}")" = "$(git rev-parse fork/master)"`.
  6. `git push --no-follow-tags fork "$TAG"`.
  7. Watch the release run, then poll the tap: `gh run list -R cameronsjo/homebrew-tap --workflow "Update Formula" --limit 3`.
  8. End on one verdict line.
- [x] Run it for `v0.9.0-palette.1`.
- [x] Verify end to end (see the Verification table — `brew fetch`, not `brew info`).
- [x] Write `docs/` or the script header with the recovery rule, because this is the judgment call the script cannot make:
  - **A palette tag is never reused.** A bad binary is fixed by cutting `v0.9.0-palette.2`.
  - **`tap-bump` failed but `release` succeeded** → `gh run rerun --failed <release-run-id>`. The receiver is idempotent (`git diff --cached --quiet && exit 0`), so re-firing is safe. But the job downloads `BUILD_INFO.txt` from an artifact with `retention-days: 7` (`release.yml:92`) — past that window the rerun dies at download and a new tag is the only path.
- [ ] Server restart is Cameron's call. The herdr **server** renders every pane, so the new binary does not take effect until it restarts, which kills every live pane. A desk item already tracks a wedged server at pid 11180 whose API socket refuses connections — worth folding into the same restart.

## Task 3 — Fit CI to the fork

Separate PR, after the release. Gate rather than delete wherever the syntax allows — matches the four workflows already carrying the line, and merges clean on every future sync.

- [x] `.github/workflows/nix.yml` — add `if: github.repository == 'herdrdev/herdr'` at **job** level. The fork distributes no Nix package.
- [x] `.github/workflows/windows-arm64.yml` — same gate. No Windows installer is shipped.
- [x] `.github/workflows/ci.yml`, `windows-conpty-package` job — same gate. Verified: nothing `needs:` this job, so gating it breaks no dependency.
- [x] `.github/workflows/ci.yml` — remove the `- os: windows-latest` matrix row from `check`. This one is a deletion because `matrix` is not an available context for a job-level `if:`, so a per-row gate is genuinely impossible. **Say the tradeoff out loud in the commit body:** `ci.yml` is not in `FORK_OWNED` (`scripts/sync-upstream-ci.sh:17`), so upstream editing that matrix block will conflict and abort a future sync. That is fail-loud and correct — but with the cron gone it surfaces only on a manual dispatch. Leave the `if: matrix.kind == 'windows'` steps in place; with no windows row they never fire, and removing them only widens the conflict surface.
- [x] `.github/workflows/sync-upstream.yml` — remove the `schedule:` trigger, **keep `workflow_dispatch`** so a hermes auto-maintainer has an entry point. Header comment records why (5/5 scheduled failures; conflicts are the normal case) so a future sync does not helpfully restore it.
- [x] **Replace the drift signal the cron was providing.** The 5 red runs were the detection half working; only the PR-opening half failed. Removing the cron leaves nothing watching upstream — which is how the tap got to `0.8.2`. Commit `scripts/upstream-drift.sh`: fetch `origin`, print `git rev-list --count fork/master..origin/master` and the newest upstream tag, exit non-zero when behind. Name it in the fork README so it is findable cold.
- [x] **Harden the release trigger**, in this same PR (it is a workflow change, and it closes the silent-downgrade path in the Panel note): narrow `release.yml`'s trigger from `v*` to `v*-palette.[0-9]*`, and make the palette segment **mandatory** in `TAG_RE` at `scripts/fork_release_notes.py:21`. Update that file's unit test alongside.
- [x] Validate before committing — `just check` per this repo's `CLAUDE.md`, and propose the commit message for Cameron's alignment first, which that file also requires. Lowercase conventional, no AI co-author line, no closing keywords. No `refs #<issue>` line: there is no issue for this work, stated here so the omission does not read as a miss.

## Verification

Each row is a command whose failure mode was checked, not just its success.

| Claim | Proving command | Why this one |
|---|---|---|
| Sync passes tests | `just check` in the worktree, exit 0 | Run from the worktree, or it tests the pre-merge tree |
| Sync landed on the fork | `git branch -r --contains f8fe3131` names `fork/master` | Push output cannot prove this |
| Tag is on the right commit | `test "$(git rev-parse v0.9.0-palette.1^{})" = "$(git rev-parse fork/master)"` | `git tag` defaults to HEAD |
| No upstream tag leaked | the scoped `grep … && echo LEAK \|\| echo CLEAN` from Task 2 | An unscoped `ls-remote` lists ~80 fork-creation tags and always looks red |
| Release published | `gh run list --repo cameronsjo/herdr --workflow Release --limit 3` | — |
| Tap receiver ran | `gh run list -R cameronsjo/homebrew-tap --workflow "Update Formula" --limit 3` | A dispatch 204 means queued, not applied |
| Tap bumped **and the asset is real** | `brew update && brew fetch herdr` | `brew info` reports the formula's version string whether or not the asset exists or hashes correctly; `brew fetch` downloads and verifies the sha256 |
| The palette build is what installed | `brew upgrade herdr` then a palette-specific check on the binary | `herdr --version` prints `Cargo.toml`'s `0.9.0` — it cannot tell palette.1 from palette.2, or a fork build from an upstream one |
| CI got smaller | `gh api repos/cameronsjo/herdr/actions/runs/<id>/timing` on a later **code** PR | Gated jobs still appear as `skipped` in `--json jobs`, so asserting absence is false; and `--json jobs` carries no billable time. Measure a code PR — the Task 3 PR edits `nix.yml` and `windows-arm64.yml`, which sit inside those workflows' own `paths:` filters, making it the least representative run |
| Nothing broke | that run green on `check (ubuntu-latest)` and `check (macos-latest)` | — |

## Loose ends, not work

- **`label-next-release-issues.yml` is ungated** and closes fork issues named by `refs #N` in any commit pushed to master, including upstream's. Checked: the last sync's refs (#10–#25) were the fork's own and closed correctly, and the three incoming commits carry `refs #3778` / `refs #3731`, which do not exist on the fork — the workflow warns and continues (`:94-96`), fails safe. The hazard is a *number collision* as upstream references low issue numbers the fork also has. A precise fix is to scan only commits not reachable from `origin/master`. Not urgent; worth an issue.
- `distribution/latest.json` and `preview.json` describe upstream's 5-platform releases at `herdrdev/herdr`. Nothing reads them.
- `.github/MAINTAINERS` and `APPROVED_CONTRIBUTORS` list upstream's people and the issue templates point at upstream's discussions — misleading to anyone filing on the fork's own tracker. Only the gated-off `pr-gate.yml` reads them.
- `just check` runs `windows-lint`, cross-compiling clippy for Windows on Cameron's laptop. Not in CI's unix path (`ci.yml` runs `just ci` there) — but `ci.yml`'s **windows row runs `just check`**, so that note only becomes fully true once Task 3 removes the row.

---

## Execution status (2026-09-08)

`v0.9.0-palette.1` is published and `cameronsjo/homebrew-tap` carries it;
`brew fetch` verified the asset downloads and hashes. Outstanding: `brew upgrade`
on this machine and the herdr server restart, both held for Cameron.

- PR #57 — sync merged (`e563360d`)
- PR #58 — release script, open
- PR #59 — CI fitting, open

## Deviations

- **The tap was at `0.8.2-palette.5`, not `0.8.2`.** Cut 2026-09-05. The gap the
  plan describes is real — master carried upstream `0.9.0` with no matching tag —
  but the pipeline was not cold.
- **`just check` fails locally on two `live_handoff` tests.** Both reproduce on
  pre-merge `b4de528a` and `tests/live_handoff.rs` is not in the merge diff, so
  they are a local-environment artifact, not a regression. `check (ubuntu-latest)`
  runs them under `all()` and passes. Note `check (macos-latest)` runs
  `not binary(live_handoff)`, so it cannot corroborate this either way.
- **The plan's "CI got smaller" proving command measures nothing here.**
  `gh api .../timing` reports `total_ms: 0` for every platform because this is a
  public repo and minutes are not billable. Baseline captured as wall-clock
  per-job duration instead: 23.2 min total, Windows 12.7 of it.
- **Two reviewers found the release script's guards fail open.** Every remote
  check was `git ls-remote | grep -q`, which under `pipefail` returns grep's 1
  when the remote call fails — so a network or auth failure read as "no match"
  and the script proceeded to push. The tap verification was inert: it read the
  tap's newest run with no correlation to this release, and the tap carries four
  formulae, so it matched a prior success on iteration one. Both fixed; the tap
  check now reads the formula's own bytes.
- **`fork_release_notes.py` had no unit test to update.** The plan assumed one.
  Wrote `scripts/test_fork_release_notes.py` (10 tests) and registered it in the
  justfile's `maintenance-test` recipe — which is a line upstream edits whenever
  it adds a test, so it is a future conflict point.
- **A session ran the release script intending to exercise its guards, and cut
  the release.** The script had no dry-run mode. Added `--dry-run`. The same run
  was then SIGPIPE'd by being piped into `sed -n '1,40p'`, which killed it after
  the push — and `--watch`, the resume path, died at the already-published guard,
  the exact state it exists for. Both fixed and both then used for real.
- **Release cut before PR #59 landed**, so it shipped with the loose `v*` release
  trigger still active. No exposure: the tag pushed was palette-shaped and
  `--no-follow-tags` prevented any upstream tag leaking alongside it.

## Learnings

- A check that cannot go red reads as proof. The tap verification, the
  `gh api .../timing` row, and the first `just check` run (which reported the
  exit status of a trailing `echo`) were all greens that could not have failed.
- Piping a long-running script into a head-like filter SIGPIPEs it. `sed -n '1,40p'`
  closed the pipe and killed the release script mid-flight.
