## Context

`agent.start` does not spawn a process. It types a command line into a pane that is sitting at its interactive shell prompt (`interactive_shell_command`, per platform) and then waits for the agent to be detected. So "env for the agent process only" has to be expressed in the typed command, in the syntax of whichever shell is in the pane.

## Goals / Non-Goals

**Goals:** the variables reach the agent process and its children, and are gone from the pane's shell once the agent exits; values are taken literally; callers can tell whether a server honors `env`.

**Non-Goals:** hiding values from the pane itself. The typed command is visible on screen and can land in the shell's history, exactly like `agent start` arguments today.

## Decisions

- **Prefix assignment for POSIX-family shells.** `sh`, `bash`, `dash`, `zsh`, `ksh`, `mksh`, `fish` (3.1+) and `nu` get `KEY='value' agent args`. This scopes the variable to one command and still resolves shell aliases and functions for the agent name, which `env KEY=value agent` would bypass. `csh`, `tcsh`, `elvish` and `xonsh` have no prefix syntax, so they get `env KEY='value' agent args`.
- **PowerShell saves and restores.** PowerShell has no per-command env syntax. The launch runs inside `& { ... try { set; agent } finally { restore } }`, which records each variable's previous value (or absence) and puts it back when the agent exits. On Windows the same wrapper goes inside the script that cmd.exe already runs through `powershell.exe -EncodedCommand`.
- **Values are always quoted for the target shell.** `~` and `$VAR` in a value are not expanded in the pane; callers expand them before calling.
- **Capability flag, not a new method.** `agent_start_env` in `ServerCapabilities` (default `false`) lets the CLI, and any raw API client, refuse to send `env` to a server that would ignore it. A new method name was the alternative; it would duplicate `agent.start`'s readiness and rollback behavior for one optional field. Old clients talking to a new server are unaffected because the field is optional.
- **A redacting map type.** `env` is a JSON object on the wire, the same shape as `pane.split`'s `env`. In Rust it is a newtype whose `Debug` prints keys only, so a stray `{:?}` of a request cannot leak a value. Errors name a key only when the key itself was valid; a malformed key is not echoed, since it may be a value in the wrong place.

## Risks / Trade-offs

- [Values are visible in the pane and may enter shell history] → recorded here and in the `docs/fork/CHANGES.md` entry; a leading-space trick only works when the user's shell is configured for it, so it was not added.
- [PowerShell treats an empty value as "remove the variable"] → `KEY=` under PowerShell unsets `KEY` for the agent rather than setting it empty.
- [Nushell's single-quoted strings have no escape, so a value containing `'` is mis-quoted there] → the same limitation already applies to `agent start` arguments in nu.
- [Keys such as `HERDR_PANE_ID` can be overridden] → allowed deliberately; the caller owns the agent's environment.
