## Why

`docker-check.sh` read only nextest's `FAIL` lines. A test killed by a signal (`SIGABRT` from a stack overflow or double panic, `SIGSEGV`), stopped by a timeout (`TIMEOUT`), aborted on Windows (`ABORT`), or leaking a handle (`LEAK-FAIL`) is reported with that status instead. Such a run parsed as zero failures, and the script printed `docker-check: PASS`.

## What Changes

- Every nextest failing status counts as a failure, and goes through the same allowlist and retry path as `FAIL`.
- A nextest run that exits non-zero with no failing test recognized fails the check.

## Impact

- `scripts/docker-check.sh` (`classify_nextest_failures` and the main body), `scripts/test_docker_check.py`, `docs/fork/CHANGES.md`.
