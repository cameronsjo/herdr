# Sync upstream into the fork

Codex will merge upstream `herdrdev/herdr` master at `792b2baa` into the fork based on `c8e54868`, preserve fork behavior, validate, and open a PR against `cameronsjo/herdr:master`. Cameron approved this approach and the merge subject `chore: sync upstream master into the fork`.

Use an isolated worktree. Preserve the fork README, palette, grouped sidebar and trailing tokens, workspace merge/move operations, integration behavior, and fork-only distribution gates. No upstream writes or release actions. Merge rather than reset/rebase so existing fork history survives.

This is release-risk integration across client state, input projection, and API behavior. Existing palette, sidebar, workspace merge, endpoint contract and identity tests are characterization coverage. Review both conflicts and clean merges that cross these features; use a bounded agent review before delivery.

- [x] Fetch both remotes, verify fork account and preview conflicts.
- [x] Merge and resolve conflicts, inspect behavior overlap.
- [x] Review correctness and simplification; fix findings.
- [x] Run `just check` and applicable scaling validation.
- [x] Prepare the validated merge for fork PR delivery.

Delivery target: `cameronsjo/herdr:master`, branch `chore/sync-upstream-20260907`. Codex will push this merge and open the PR; GitHub records publication and check status.

Validation results and deviations will be recorded below.

Cameron additionally authorized simplifying fork features to reduce future sync cost. Codex is reusing upstream navigation and generic agent-list rendering, isolating palette input in its own module, and composing token alignment with upstream conditional styles. Fork moves remain server-local.

Review identified and corrected cross-endpoint drag targeting, deferred active-agent dragging, stale tab-drop targets at release, and missing advertised pane/plugin methods. Destination-based tab moves now negotiate `tab.move_to_destination`; legacy fork destinations remain accepted and index-only reorders return upstream-compatible `tab_list`. Frozen codec values and existing method digests remain unchanged.

Validation setup: host is sjomba. Shared Docker VM has 2 GB RAM and killed compilation; a temporary Colima profile `herdr-sync-20260907` provides 6 GB without activating its context or restarting shared services. ARM Linux Zig SIMD compilation fails before Rust; validation uses `LIBGHOSTTY_VT_SIMD=false` and reports that limitation. Dockerfile now uses a pinned checksum-verified prebuilt nextest to avoid building the test runner and removes duplicate ARG declarations that erased default versions on the legacy builder.

The final independent read-only review found no remaining actionable correctness issues in endpoint ownership, move compatibility, capability negotiation, headless reconciliation, or renderer integration. Docker validation now uses `--init` to reap orphaned test children; all three process-cleanup tests passed without exclusions. A metadata test used a 1 ms TTL during setup and raced under load; its setup window is now 60 seconds while expiry is still invoked explicitly.

`just check` passed in the isolated ARM Linux container: 3,415 Rust tests (2 intentionally ignored), 109 maintenance tests, 8 architecture tests, integration/plugin suites, Windows-target clippy, and docs contracts. The architecture guard now verifies that the shared renderer supplies the same precomputed heights to scroll metrics and drawn rows; two deliberately inconsistent variants were rejected. SIMD remained disabled for this run.

`just bench-render-scale` passed (2 profiles). At fixed 120x40 geometry, grouped-agent client composition for 1 to 15 background panes measured 230 to 251 microseconds median (+9%), 243 to 305 microseconds p95 (+26%). Active panes measured 225 to 232 microseconds median (+3%), 235 to 240 microseconds p95 (+2%). Ungrouped medians were 224 to 261 microseconds background and 230 to 237 microseconds active. These are ARM Linux VM samples with SIMD disabled, not native macOS or release throughput measurements.

Draft PR: https://github.com/cameronsjo/herdr/pull/56. Initial GitHub Linux, macOS, Nix, Windows packaging, and ARM64 installer checks passed. Windows passed 2,972 Rust tests then exposed the Unix-only Docker harness tests being invoked on Windows; those tests now declare their POSIX host requirement, matching the Unix installer tests. Codex verified the maintenance suite on macOS before updating the merge; no runtime code changed after the full local validation. CodeRabbit skipped the draft.
