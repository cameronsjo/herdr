# fork-maintenance Specification

## Purpose
Fork maintenance covers the scripts and rules the fork's operator uses to keep `cameronsjo/herdr` current with upstream, verify it, and release it, and the ledger discipline that records every divergence.

## Requirements

### Requirement: Every fork change is ledgered and every sync re-checks it
Every fork pull request MUST add an entry to `docs/fork/CHANGES.md` naming the PR, the files, what upstream behavior it replaces, and a regression check. Every upstream sync MUST re-run the regression checks of every ledger section still in the fork before the sync merge is pushed, and MUST record each conflicted file and which side's behavior was kept.

#### Scenario: A fork PR lands
- **WHEN** a pull request changes fork behavior
- **THEN** the same pull request adds its `docs/fork/CHANGES.md` entry with a regression check

#### Scenario: A sync conflicts
- **WHEN** an upstream sync merge conflicts in a file that carries fork behavior
- **THEN** the ledger gains a conflict-resolution record for that sync naming the file and the behavior kept

### Requirement: sync-upstream merges upstream in its own worktree
`scripts/sync-upstream.sh` SHALL fetch the upstream and fork remotes (`ORIGIN_REMOTE`, default `origin`; `FORK_REMOTE`, default `fork`), failing if either is missing, then create or reuse the worktree `../herdr-worktrees/sync-upstream-<YYYYMMDD>` on a branch of that name from the fork's `master`, and merge upstream `master` into it. A `README.md` conflict MUST be resolved by keeping the fork's copy.

#### Scenario: Clean merge
- **WHEN** the script runs and upstream merges without conflicts
- **THEN** the merge commit lands in the worktree and the next steps are printed

### Requirement: sync-upstream stops on conflicts and prints next steps
Any conflict other than `README.md` MUST stop `scripts/sync-upstream.sh` with a non-zero exit, listing every unmerged path including rename and delete conflicts, and the worktree path. A completed merge SHALL print the next steps: run `docker-check.sh` from the worktree, push with `--no-follow-tags`, fast-forward `master`, then remove the worktree and branch.

#### Scenario: Conflicts left for a person
- **WHEN** upstream conflicts with the fork in `src/app/ids.rs`
- **THEN** the script exits non-zero and lists `src/app/ids.rs` with the worktree path

### Requirement: sync-upstream resumes after hand-resolved conflicts
Rerunning `scripts/sync-upstream.sh` after the conflicts are resolved and staged by hand SHALL reuse the existing worktree and complete the merge commit, instead of refusing the staged changes. It MUST still refuse to run when the worktree has uncommitted changes and no merge is in progress.

#### Scenario: Resume after resolving
- **WHEN** a previous run stopped on conflicts and every conflict has since been resolved and staged
- **THEN** rerunning the script commits the merge and prints the next steps

#### Scenario: Unrelated dirt
- **WHEN** the sync worktree has uncommitted changes and no merge in progress
- **THEN** the script exits with an error about unrelated uncommitted changes

### Requirement: docker-check runs the Rust gate in a container
`scripts/docker-check.sh` SHALL run `cargo fmt --check`, clippy and nextest in a Linux container, then the maintenance script unit tests natively, and print `docker-check: PASS` when all pass. It MUST fail fast when Docker is missing or unreachable. The Zig prebuild MUST run on a bounded CPU set (`ZIG_BUILD_CPUSET`, default `0-1`) and the compile with bounded jobs (`CARGO_BUILD_JOBS`, default `2`) so a small Docker VM does not kill it.

#### Scenario: Docker not running
- **WHEN** the Docker daemon is not reachable
- **THEN** the script exits 1 with an error asking whether Colima or Docker Desktop is running

#### Scenario: Clean tree
- **WHEN** the tree builds and every test passes
- **THEN** the script ends with `docker-check: PASS`

### Requirement: docker-check retries unexpected failures once
A nextest failure listed on the script's known-environment allowlist MUST NOT fail the run. Any other failure SHALL be retried once, alone, scoped to its binary and test name, and reported as a real failure only if it fails again; a retry filter that matches no test MUST count as a failure, never a pass. Failure lines MUST be parsed correctly whether nextest's progress counter is padded or not.

#### Scenario: Load flake
- **WHEN** a test outside the allowlist fails in the full run and passes when retried alone
- **THEN** the run is not failed and a note says it passed on retry

#### Scenario: Real regression
- **WHEN** a test outside the allowlist fails in the full run and again on retry
- **THEN** the script exits 1 listing it under unexpected test failures

### Requirement: The release cut script refuses unsafe tags
`scripts/cut-fork-release.sh [--dry-run|--watch] vMAJOR.MINOR.PATCH-palette.N` MUST refuse a tag of any other shape, a fork remote that is not exactly `cameronsjo/herdr`, modified tracked files, a tag already published, a tag that exists only locally (saying to delete it and rerun), a plain upstream `vX.Y.Z` tag on the fork at or above the release's base version, and missing release credentials. `--dry-run` MUST run every check and push nothing.

#### Scenario: Dry run
- **WHEN** `scripts/cut-fork-release.sh --dry-run v0.9.0-palette.1` runs and every precondition holds
- **THEN** it prints `PASS` and that nothing was tagged or pushed

#### Scenario: Wrong tag shape
- **WHEN** the script is given `v0.9.0`
- **THEN** it refuses the tag before touching any remote

#### Scenario: Leaked upstream tag
- **WHEN** the fork carries a plain `v0.9.1` tag and `v0.9.0-palette.2` is being cut
- **THEN** the script refuses and names the leaked tag

### Requirement: The release cut script tags fork master and verifies the tap
Without a flag, `scripts/cut-fork-release.sh` MUST tag `fork/master`, never `HEAD`, push only that tag with `--no-follow-tags`, watch the Release run, and confirm the Homebrew tap formula carries the new version and sha256. `--watch` SHALL rejoin a tag already pushed, and a failed Release run MUST be reported with the instruction to cut the next counter rather than re-push.

#### Scenario: Rejoin a cut
- **WHEN** `scripts/cut-fork-release.sh --watch v0.9.0-palette.1` runs for a published tag
- **THEN** it skips the cut checks and resumes at watching the Release run

#### Scenario: Tap not bumped
- **WHEN** the release publishes but the tap formula does not carry the new version within 5 minutes
- **THEN** the script fails and prints how to rerun the failed job

### Requirement: The palette live check refuses a run that cannot show the feature
`scripts/palette-live-check.sh` SHALL require a controlling terminal and MUST refuse to start when the debug binary is missing, when a watched palette source is missing or newer than the binary (`Stale binary:`), when no plugin registry exists to seed its throwaway config, when the binary would not read that config, or when its override's plugin id is not one the binary lists (`no plugin with the id`). It MUST never write the real config.

#### Scenario: Stale build
- **WHEN** `src/client/shell/palette.rs` is newer than `target/debug/herdr`
- **THEN** the script refuses with `Stale binary:` and a build hint

#### Scenario: Display name used as the plugin id
- **WHEN** the override names `collie:uninstall` while the plugin's id is `herdr.collie`
- **THEN** the script refuses with `no plugin with the id`

### Requirement: Zig dependencies can be prefetched
`just zig-prefetch` (`scripts/prefetch_zig_deps.py`) SHALL fill the Zig package cache for the vendored libghostty-vt build, walking nested `build.zig.zon` dependencies and skipping packages already cached. It MUST refuse a dependency whose URL is not https or whose hash or URL could inject options or escape the work directory, MUST trust a download only once `zig fetch` computes exactly the pinned hash, and MUST exit non-zero when any dependency cannot be fetched.

#### Scenario: Proxy blocks Zig's own fetch
- **WHEN** `cargo build` fails in `build.rs` with an HTTP error from a `build.zig.zon` and `just zig-prefetch` is run
- **THEN** every dependency lands in the Zig cache, hash-verified, and the next build gets past the Zig fetch

#### Scenario: Hostile dependency hash
- **WHEN** a nested `build.zig.zon` names a dependency whose hash contains `../`
- **THEN** the script refuses that dependency and exits non-zero
