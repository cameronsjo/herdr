## MODIFIED Requirements

### Requirement: sync-upstream resumes after hand-resolved conflicts
Rerunning `scripts/sync-upstream.sh` after the conflicts are resolved and staged by hand SHALL reuse the existing worktree and complete the merge commit, instead of refusing the staged changes, even when the rerun falls on a later day than the run that stopped. It MUST still refuse to run when the worktree has uncommitted changes and no merge is in progress, and when more than one sync worktree has a merge in progress.

#### Scenario: Resume after resolving
- **WHEN** a previous run stopped on conflicts and every conflict has since been resolved and staged
- **THEN** rerunning the script commits the merge and prints the next steps

#### Scenario: Resume on a later day
- **WHEN** a run stopped on conflicts in `sync-upstream-20261022`, the conflicts were resolved and staged, and the script is rerun on 2026-10-23
- **THEN** it finishes the merge in `sync-upstream-20261022` and creates no new worktree

#### Scenario: Unrelated dirt
- **WHEN** the sync worktree has uncommitted changes and no merge in progress
- **THEN** the script exits with an error about unrelated uncommitted changes
