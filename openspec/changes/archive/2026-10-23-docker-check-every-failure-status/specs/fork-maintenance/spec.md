## ADDED Requirements

### Requirement: docker-check counts every failing test status
`scripts/docker-check.sh` SHALL treat a test nextest reports as killed by a signal, timed out, aborted, or leaking (`SIG*`, `TIMEOUT`, `ABORT`, `LEAK-FAIL`) as a failure, exactly like `FAIL`. A nextest run that exits non-zero with no failing test recognized MUST fail the check, never print `docker-check: PASS`.

#### Scenario: Test killed by a signal
- **WHEN** a test outside the allowlist is reported `SIGABRT` in the full run and again on retry
- **THEN** the script exits 1 listing it under unexpected test failures

#### Scenario: Unrecognized failure
- **WHEN** nextest exits non-zero and no line of its output is recognized as a failing test
- **THEN** the script exits 1 and does not print `docker-check: PASS`
