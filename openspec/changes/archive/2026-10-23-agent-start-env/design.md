## Context

`agent.start` does not spawn a process. It types a command line into a pane that is sitting at its interactive shell prompt (`interactive_shell_command`, per platform) and then waits for the agent to be detected. So "env for the agent process only" has to be expressed in the typed command, in the syntax of whichever shell is in the pane.

## Goals / Non-Goals

**Goals:** the variables reach the agent process and its children, and are gone from the pane's shell once the agent exits; values are taken literally; callers can tell whether a server honors `env`.

**Non-Goals:** hiding values from the pane itself. The typed command is visible on screen and can land in the shell's history, exactly like `agent start` arguments today.

## Decisions

- **Prefix assignment, only where the quoting is verified.** `sh`, `bash`, `dash`, `zsh`, `ksh` and `mksh` get `KEY='value' agent args` with POSIX single quotes (`'` written as `'\''`). `fish` (3.1+) gets the same prefix form with fish's own quoting: inside single quotes fish treats `\'` and `\\` as escapes, so both are escaped, and a value ending in `\` stays closed. The prefix form scopes the variable to one command and still resolves shell aliases and functions for the agent name, which `env KEY=value agent` would bypass. Real-shell tests run each of these (when installed) with adversarial values and require the agent to see them byte for byte.
- **Every other Unix shell is refused.** `nu`, `xonsh`, `elvish`, `csh` and `tcsh` each need a quoting scheme of their own, and POSIX quoting there was shown to execute part of a value (fish, nu, elvish, xonsh) or to corrupt it (csh-family `!` history expansion inside single quotes). Rather than ship unverified quoters, `agent.start` fails with `agent_env_unsupported_shell` in those panes before typing anything. A launch without `env` is unaffected.
- **PowerShell saves and restores.** PowerShell has no per-command env syntax. The launch runs inside `& { ... try { set; agent } finally { restore } }`, which records each variable's previous value (or absence) in an ordinal, case-sensitive dictionary and puts it back when the agent exits. On Windows the same wrapper goes inside the script that cmd.exe already runs through `powershell.exe -EncodedCommand`.
- **Values reach the agent literally.** In the supported shells, `~` and `$VAR` in a value are not expanded in the pane; callers expand them before calling.
- **Values are restricted to what can be typed safely.** Control characters (Cc, including U+0085), U+2028, U+2029 and format characters (Cf, such as bidi overrides and zero-width characters) are refused, so the command echoed in the pane reads the same as what runs.
- **Capability flag, not a new method.** `agent_start_env` in `ServerCapabilities` (default `false`) lets the CLI, and any raw API client, refuse to send `env` to a server that would ignore it. A new method name was the alternative; it would duplicate `agent.start`'s readiness and rollback behavior for one optional field. Old clients talking to a new server are unaffected because the field is optional.
- **A redacting map type.** `env` is a JSON object on the wire, the same shape as `pane.split`'s `env`. In Rust it is a newtype whose `Debug` prints keys only, so a stray `{:?}` of a request cannot leak a value. Errors name a key only when the key itself was valid; a malformed key is not echoed, since it may be a value in the wrong place. Its `Deserialize` is hand-written so an `env` of the wrong JSON type is refused without serde quoting the value in the `invalid_request` message.

## Risks / Trade-offs

- [Values are visible in the pane and may enter shell history] → recorded here and in the `docs/fork/CHANGES.md` entry; a leading-space trick only works when the user's shell is configured for it, so it was not added.
- [PowerShell treats an empty value as "remove the variable"] → `KEY=` under PowerShell unsets `KEY` for the agent rather than setting it empty.
- [`--env` is unavailable in nu, xonsh, elvish, csh and tcsh panes] → refused with a clear error; a verified quoter per shell can lift it later. `agent start` arguments still use POSIX quoting in those shells, an upstream limitation this change does not touch.
- [A restored or resumed agent loses its launch env] → env is deliberately not persisted, since it may hold credentials. Resume relaunches the agent without it; a follow-up issue tracks how to handle this.
- [Values also appear in the `herdr` CLI process's own argv while the call runs] → visible to `ps` for that time; callers that care can use the API directly.
- [Keys such as `HERDR_PANE_ID` can be overridden] → allowed deliberately; the caller owns the agent's environment.
