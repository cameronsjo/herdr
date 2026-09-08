# Sync upstream into the fork

Codex merged upstream `herdrdev/herdr` master at `792b2baa` into the fork based on `c8e54868`, preserved fork behavior, validated, and opened a PR against `cameronsjo/herdr:master`. Cameron approved this approach and the merge subject `chore: sync upstream master into the fork`.

Use an isolated worktree. Preserve the fork README, palette, grouped sidebar and trailing tokens, workspace merge/move operations, integration behavior, and fork-only distribution gates. No upstream writes or release actions. Merge rather than reset/rebase so existing fork history survives.

This is release-risk integration across client state, input projection, and API behavior. Existing palette, sidebar, workspace merge, endpoint contract and identity tests are characterization coverage. Review both conflicts and clean merges that cross these features; use a bounded agent review before delivery.

- [x] Fetch both remotes, verify fork account and preview conflicts.
- [x] Merge and resolve conflicts, inspect behavior overlap.
- [x] Review correctness and simplification; fix findings.
- [x] Run `just check` and applicable scaling validation.
- [x] Publish PR #56, verify CI, and merge with Cameron's approval.
- [x] Fast-forward the shared checkout and remove the completed task worktree and branches.

Status: complete. Cameron authorized merging PR #56; GitHub records merge `e839cf59` on `cameronsjo/herdr:master`. The shared checkout has been fast-forwarded to that merge.

Next action: none required for this sync; choose the next task from `master`.

Validation results and deviations follow.

Cameron additionally authorized simplifying fork features to reduce future sync cost. Codex is reusing upstream navigation and generic agent-list rendering, isolating palette input in its own module, and composing token alignment with upstream conditional styles. Fork moves remain server-local.

Review identified and corrected cross-endpoint drag targeting, deferred active-agent dragging, stale tab-drop targets at release, and missing advertised pane/plugin methods. Destination-based tab moves now negotiate `tab.move_to_destination`; legacy fork destinations remain accepted and index-only reorders return upstream-compatible `tab_list`. Frozen codec values and existing method digests remain unchanged.

Validation setup: host is sjomba. Shared Docker VM has 2 GB RAM and killed compilation; a temporary Colima profile `herdr-sync-20260907` provides 6 GB without activating its context or restarting shared services. ARM Linux Zig SIMD compilation fails before Rust; validation uses `LIBGHOSTTY_VT_SIMD=false` and reports that limitation. Dockerfile now uses a pinned checksum-verified prebuilt nextest to avoid building the test runner and removes duplicate ARG declarations that erased default versions on the legacy builder.

The final independent read-only review found no remaining actionable correctness issues in endpoint ownership, move compatibility, capability negotiation, headless reconciliation, or renderer integration. Docker validation now uses `--init` to reap orphaned test children; all three process-cleanup tests passed without exclusions. A metadata test used a 1 ms TTL during setup and raced under load; its setup window is now 60 seconds while expiry is still invoked explicitly.

`just check` passed in the isolated ARM Linux container: 3,415 Rust tests (2 intentionally ignored), 109 maintenance tests, 8 architecture tests, integration/plugin suites, Windows-target clippy, and docs contracts. The architecture guard now verifies that the shared renderer supplies the same precomputed heights to scroll metrics and drawn rows; two deliberately inconsistent variants were rejected. SIMD remained disabled for this run.

`just bench-render-scale` passed (2 profiles). At fixed 120x40 geometry, grouped-agent client composition for 1 to 15 background panes measured 230 to 251 microseconds median (+9%), 243 to 305 microseconds p95 (+26%). Active panes measured 225 to 232 microseconds median (+3%), 235 to 240 microseconds p95 (+2%). Ungrouped medians were 224 to 261 microseconds background and 230 to 237 microseconds active. These are ARM Linux VM samples with SIMD disabled, not native macOS or release throughput measurements.

Draft PR: https://github.com/cameronsjo/herdr/pull/56. Initial GitHub Linux, macOS, Nix, Windows packaging, and ARM64 installer checks passed. Windows passed 2,972 Rust tests then exposed the Unix-only Docker harness tests being invoked on Windows; those tests now declare their POSIX host requirement, matching the Unix installer tests. Codex verified the maintenance suite on macOS before updating the merge; no runtime code changed after the full local validation. CodeRabbit skipped the draft.

Session closeout:

- `gh pr view 56 --repo cameronsjo/herdr --json state,mergeCommit,mergedAt` confirmed the merge. `gh pr checks 56 --repo cameronsjo/herdr` confirmed Linux, macOS, Windows, Nix, packaging, and installer checks passed on `098d7491`.
- CodeRabbit did not perform a review: it initially skipped the draft, then skipped the ready PR because 359 files exceeded its 150-file limit. The independent Codex review and test evidence above remain the review record; no bot verdict is claimed.
- The disposable validation VM, task worktree, and local/remote task branches were removed. Unrelated `.pi/tasks/` work remains untouched.
- The ARM Linux SIMD build issue is retained here as a validation-environment limitation; the platform CI checks passed. Manual installation, interactive smoke testing, and release publication were outside this sync and are not pending session work.
- All requested sync and simplification work is complete; no follow-up issue or deferred implementation remains.
