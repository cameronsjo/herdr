## MODIFIED Requirements

### Requirement: sync-upstream merges upstream in its own worktree
`scripts/sync-upstream.sh` SHALL fetch the upstream and fork remotes (`ORIGIN_REMOTE`, default `origin`; `FORK_REMOTE`, default `fork`), failing if either is missing, then create or reuse the worktree `../herdr-worktrees/sync-upstream-<YYYYMMDD>` on a branch of that name from the fork's `master`, and merge upstream `master` into it with `--no-commit`. Before committing, it MUST restore every file in its `FORK_OWNED` list from the fork's tip, whether or not it conflicted, and that list MUST match the one in `scripts/sync-upstream-ci.sh`.

#### Scenario: Clean merge
- **WHEN** the script runs and upstream merges without conflicts
- **THEN** the merge commit lands in the worktree and the next steps are printed

#### Scenario: Upstream edits a fork-owned file cleanly
- **WHEN** upstream changes `.github/workflows/release.yml` in a region the fork never touched
- **THEN** the merge commit carries the fork's copy unchanged

#### Scenario: Upstream conflicts in the README
- **WHEN** upstream and the fork both change `README.md`
- **THEN** the script keeps the fork's copy and does not list `README.md` as a conflict

### Requirement: sync-upstream stops on conflicts and prints next steps
Any conflict outside `FORK_OWNED` MUST stop `scripts/sync-upstream.sh` with a non-zero exit, listing every unmerged path including rename and delete conflicts, and the worktree path. A completed merge SHALL print the next steps: run `docker-check.sh` from the worktree, push with `--no-follow-tags`, fast-forward `master`, then remove the worktree and branch.

#### Scenario: Conflicts left for a person
- **WHEN** upstream conflicts with the fork in `src/app/ids.rs`
- **THEN** the script exits non-zero and lists `src/app/ids.rs` with the worktree path
