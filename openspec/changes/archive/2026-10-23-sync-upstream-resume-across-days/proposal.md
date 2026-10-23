## Why

`sync-upstream.sh` names its branch and worktree after the day it runs. A sync that stopped on conflicts and was resumed after midnight looked for today's worktree, did not find the hand-resolved one, created a fresh worktree from `fork/master`, hit the same conflicts again and exited 1. The resolved merge was left orphaned.

## What Changes

- Before using today's name, the script looks for a `sync-upstream-*` worktree with a merge in progress and resumes it, whatever day it is named for.
- Two such worktrees at once is an error that names both.

## Impact

- `scripts/sync-upstream.sh`, new `scripts/test_sync_upstream.py`, `justfile` (`maintenance-test`).
- `docs/fork/CHANGES.md`.
