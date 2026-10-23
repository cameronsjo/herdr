## 1. API and validation

- [x] 1.1 Add the optional `env` object to `agent.start` params with a key-only `Debug`, and the key grammar, missing-`=` and control-character checks
- [x] 1.2 Advertise `capabilities.agent_start_env` and show it in `herdr status --json`
- [x] 1.3 Regenerate `docs/next/api/herdr-api.schema.json`

## 2. Launch

- [x] 2.1 Build the typed launch command with env scoped to the agent for POSIX-family, `env`-style and PowerShell shells, and inside the Windows encoded script
- [x] 2.2 Validate `env` in `agent.start` before any input reaches the pane, with the `invalid_agent_env` error

## 3. CLI

- [x] 3.1 Add the repeatable `--env KEY=VALUE` option to `herdr agent start`
- [x] 3.2 Refuse `--env` with `agent_start_env_unsupported` when the server lacks the capability

## 4. Verification

- [x] 4.1 Unit tests: key grammar, missing `=`, values absent from every error and `Debug`, the typed command per shell, a real `/bin/sh` run proving the agent sees the value and the shell does not, and the capability gate
- [x] 4.2 Pass `scripts/openspec.sh validate --all --strict`

## 5. Ledger

- [x] 5.1 Add the `docs/fork/CHANGES.md` entry, with its regression check

## 6. Review follow-up (PR #124)

- [x] 6.1 Limit `--env` to shells with verified quoting (sh family, fish with its own quoter, pwsh) and refuse the rest with `agent_env_unsupported_shell`
- [x] 6.2 Real-shell round-trip tests with adversarial values for every supported shell that is installed
- [x] 6.3 Refuse U+2028, U+2029 and format characters in values; keep a mistyped `env` out of `invalid_request` messages
- [x] 6.4 Record that a restored agent does not keep its launch env
